use crate::{
    correlation_id::CorrelationId,
    element::Element,
    event::{Event, EventType},
    name,
    ref_data::RefData,
    request::Request,
    service::Service,
    session_options::SessionOptions,
    Error,
};
use blpapi_sys::*;
use std::collections::HashMap;
use std::{ffi::CString, ptr};

const MAX_PENDING_REQUEST: usize = 1024;
const MAX_REFDATA_FIELDS: usize = 400;
const MAX_HISTDATA_FIELDS: usize = 25;

pub struct Session {
    ptr: *mut blpapi_Session_t,
    correlation_count: u64,
}

impl Session {
    fn from_options(options: SessionOptions) -> Self {
        let handler: Option<
            unsafe fn(*mut blpapi_Event, *mut blpapi_Session, *mut std::ffi::c_void),
        > = None;
        let dispatcher: *mut blpapi_EventDispatcher = ptr::null_mut();
        let user_data: *mut std::ffi::c_void = ptr::null_mut();
        let ptr: *mut blpapi_Session =
            unsafe { blpapi_Session_create(options.0, handler, dispatcher, user_data) };

        Session {
            ptr,
            correlation_count: 0,
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let res: i32 = unsafe { blpapi_Session_start(self.ptr) };

        Error::check(res)
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        let res: i32 = unsafe { blpapi_Session_stop(self.ptr) };

        Error::check(res)
    }

    pub fn open_service(&mut self, service: &str) -> Result<(), Error> {
        let service: CString = CString::new(service).unwrap();
        let res: i32 = unsafe { blpapi_Session_openService(self.ptr, service.as_ptr()) };

        Error::check(res)
    }

    pub fn get_service(&self, service: &str) -> Result<Service, Error> {
        let name: CString = CString::new(service).unwrap();

        let mut service: *mut blpapi_Service = ptr::null_mut();

        let res: i32 =
            unsafe { blpapi_Session_getService(self.ptr, &mut service as *mut _, name.as_ptr()) };

        Error::check(res)?;

        Ok(Service(service))
    }

    pub fn send(
        &mut self,
        request: Request,
        correlation_id: Option<CorrelationId>,
    ) -> Result<CorrelationId, Error> {
        let mut correlation_id: CorrelationId =
            correlation_id.unwrap_or_else(|| self.new_correlation_id());
        let identity: *mut blpapi_Identity = ptr::null_mut();
        let event_queue: *mut blpapi_EventQueue = ptr::null_mut();
        let request_label: *mut i8 = ptr::null_mut();
        let request_label_len: i32 = 0;

        unsafe {
            let res = blpapi_Session_sendRequest(
                self.ptr,
                request.ptr,
                &mut correlation_id.0 as *mut _,
                identity,
                event_queue,
                request_label,
                request_label_len,
            );

            Error::check(res)?;

            Ok(correlation_id)
        }
    }

    fn new_correlation_id(&mut self) -> CorrelationId {
        let id = CorrelationId::new_u64(self.correlation_count);

        self.correlation_count += 1;

        id
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { blpapi_Session_destroy(self.ptr) }
    }
}

pub struct SessionSync(Session);

impl SessionSync {
    pub fn from_options(options: SessionOptions) -> Self {
        SessionSync(Session::from_options(options))
    }

    pub fn new() -> Result<Self, Error> {
        let mut session = Self::from_options(SessionOptions::default());

        session.start()?;

        session.open_service("//blp/refdata")?;

        Ok(session)
    }

    pub fn send(
        &mut self,
        request: Request,
        correlation_id: Option<CorrelationId>,
    ) -> Result<Events, Error> {
        let _id = (&mut *self as &mut Session).send(request, correlation_id)?;

        Ok(Events::new(self))
    }

    pub fn next_event(&mut self, timeout_ms: Option<u32>) -> Result<Event, Error> {
        let mut event: *mut blpapi_Event = ptr::null_mut();

        let timeout: u32 = timeout_ms.unwrap_or(0);

        unsafe {
            let res: i32 = blpapi_Session_nextEvent(self.0.ptr, &mut event as *mut _, timeout);

            Error::check(res)?;

            Ok(Event(event))
        }
    }

    pub fn ref_data<I, R>(&mut self, securities: I) -> Result<HashMap<String, R>, Error>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
        R: RefData,
    {
        let service = self.get_service("//blp/refdata")?;
        let mut ref_data: HashMap<String, R> = HashMap::new();
        let mut iter: <I as IntoIterator>::IntoIter = securities.into_iter();

        for fields in R::FIELDS.chunks(MAX_REFDATA_FIELDS) {
            loop {
                let mut request: Request = service.create_request("ReferenceDataRequest")?;

                let mut is_empty: bool = true;

                for security in iter.by_ref().take(MAX_PENDING_REQUEST / fields.len()) {
                    request.append_named(&name::SECURITIES, security.as_ref())?;
                    is_empty = false;
                }

                if is_empty {
                    break;
                }

                for field in fields {
                    request.append_named(&name::FIELDS_NAME, *field)?;
                }

                for event in self.send(request, None)? {
                    for message in event?.messages().map(|m| m.element()) {
                        if let Some(securities) = message.get_named_element(&name::SECURITY_DATA) {
                            for security in securities.values::<Element>() {
                                let ticker: String = security
                                    .get_named_element(&name::SECURITY_NAME)
                                    .and_then(|s: Element| s.get_at(0))
                                    .unwrap_or_else(String::new);

                                if let Some(error) =
                                    security.get_named_element(&name::SECURITY_ERROR)
                                {
                                    return Err(Error::security(ticker, error));
                                }

                                let entry: &mut R = ref_data.entry(ticker).or_default();

                                if let Some(fields) = security.get_named_element(&name::FIELD_DATA)
                                {
                                    for field in fields.elements() {
                                        entry.on_field(&field.string_name(), &field);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ref_data)
    }

    pub fn hist_data<I, R>(
        &mut self,
        securities: I,
        options: HistOptions,
    ) -> Result<HashMap<String, TimeSerie<R>>, Error>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
        R: RefData,
    {
        let service = self.get_service("//blp/refdata")?;

        let mut ref_data: HashMap<String, TimeSerie<R>> = HashMap::new();
        let mut iter = securities.into_iter();

        for fields in R::FIELDS.chunks(MAX_HISTDATA_FIELDS) {
            loop {
                let mut request = service.create_request("HistoricalDataRequest")?;
                let mut is_empty = true;

                for security in iter.by_ref().take(MAX_PENDING_REQUEST / fields.len()) {
                    request.append_named(&name::SECURITIES, security.as_ref())?;
                    is_empty = false;
                }

                if is_empty {
                    break;
                }

                for field in fields {
                    request.append_named(&name::FIELDS_NAME, *field)?;
                }

                options.apply(&mut request)?;

                for event in self.send(request, None)? {
                    for message in event?.messages().map(|m| m.element()) {
                        if let Some(security) = message.get_named_element(&name::SECURITY_DATA) {
                            let ticker: String = security
                                .get_named_element(&name::SECURITY_NAME)
                                .and_then(|s: Element| s.get_at(0))
                                .unwrap_or_else(|| String::new());

                            if security.has_named_element(&name::SECURITY_ERROR) {
                                break;
                            }

                            if let Some(fields) = security.get_named_element(&name::FIELD_DATA) {
                                let entry: &mut TimeSerie<R> =
                                    ref_data.entry(ticker).or_insert_with(|| {
                                        let len: usize = fields.num_values();

                                        TimeSerie::<_>::with_capacity(len)
                                    });

                                for points in fields.values::<Element>() {
                                    let mut value = R::default();

                                    for field in points.elements() {
                                        let name = &field.string_name();

                                        if name == "date" {
                                            #[cfg(feature = "dates")]
                                            entry
                                                .dates
                                                .extend(field.get_at::<chrono::NaiveDate>(0));

                                            #[cfg(not(feature = "dates"))]
                                            entry.dates.extend(field.get_at(0));
                                        } else {
                                            value.on_field(name, &field);
                                        }
                                    }

                                    entry.values.push(value);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ref_data)
    }
}

impl std::ops::Deref for SessionSync {
    type Target = Session;
    fn deref(&self) -> &Session {
        &self.0
    }
}

impl std::ops::DerefMut for SessionSync {
    fn deref_mut(&mut self) -> &mut Session {
        &mut self.0
    }
}

pub struct Events<'a> {
    session: &'a mut SessionSync,
    exit: bool,
}

impl<'a> Events<'a> {
    fn new(session: &'a mut SessionSync) -> Self {
        Events {
            session,
            exit: false,
        }
    }

    fn try_next(&mut self) -> Result<Option<Event>, Error> {
        if self.exit {
            return Ok(None);
        }
        loop {
            let event: Event = self.session.next_event(None)?;
            let event_type: EventType = event.event_type();

            match event_type {
                EventType::PartialResponse => return Ok(Some(event)),
                EventType::Response => {
                    self.exit = true;
                    return Ok(Some(event));
                }
                EventType::SessionStatus => {
                    if event.messages().map(|m| m.message_type()).any(|m| {
                        m == *name::SESSION_TERMINATED || m == *name::SESSION_STARTUP_FAILURE
                    }) {
                        return Ok(None);
                    }
                }

                EventType::Timeout => return Err(Error::TimeOut),

                _ => (),
            }
        }
    }
}

impl<'a> Iterator for Events<'a> {
    type Item = Result<Event, Error>;

    fn next(&mut self) -> Option<Result<Event, Error>> {
        self.try_next().transpose()
    }
}

#[derive(Debug, Default)]
pub struct HistOptions {
    start_date: String,
    end_date: String,
    periodicity_adjustment: Option<PeriodicityAdjustment>,
    periodicity_selection: Option<PeriodicitySelection>,
    max_data_points: Option<i32>,
    currency: Option<String>,
}

impl HistOptions {
    pub fn new<S: Into<String>, E: Into<String>>(start_date: S, end_date: E) -> Self {
        HistOptions {
            start_date: start_date.into(),
            end_date: end_date.into(),
            ..HistOptions::default()
        }
    }

    pub fn with_periodicity_adjustment(
        mut self,
        periodicity_adjustment: PeriodicityAdjustment,
    ) -> Self {
        self.periodicity_adjustment = Some(periodicity_adjustment);
        self
    }

    pub fn with_periodicity_selection(
        mut self,
        periodicity_selection: PeriodicitySelection,
    ) -> Self {
        self.periodicity_selection = Some(periodicity_selection);
        self
    }

    pub fn with_max_points(mut self, max_data_points: i32) -> Self {
        self.max_data_points = Some(max_data_points);
        self
    }

    pub fn with_currency(mut self, currency: String) -> Self {
        self.currency = Some(currency);
        self
    }

    fn apply(&self, request: &mut Request) -> Result<(), Error> {
        let mut element = request.element();

        element.set("startDate", &self.start_date[..])?;
        element.set("endDate", &self.end_date[..])?;

        if let Some(periodicity_selection) = self.periodicity_selection {
            element.set("periodicitySelection", periodicity_selection.as_str())?;
        }

        if let Some(periodicity_adjustment) = self.periodicity_adjustment {
            element.set("periodicityAdjustment", periodicity_adjustment.as_str())?;
        }

        if let Some(max_data_points) = self.max_data_points {
            element.set("maxDataPoints", max_data_points)?;
        }

        if let Some(currency) = self.currency.as_ref() {
            element.set("currency", &**currency)?;
        }

        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct TimeSerie<R> {
    #[cfg(feature = "dates")]
    pub dates: Vec<chrono::NaiveDate>,
    pub values: Vec<R>,
}

impl<R> TimeSerie<R> {
    pub fn with_capacity(capacity: usize) -> Self {
        TimeSerie {
            dates: Vec::with_capacity(capacity),
            values: Vec::with_capacity(capacity),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PeriodicityAdjustment {
    Actual,
    Calendar,
    Fiscal,
}

impl PeriodicityAdjustment {
    pub fn as_str(self) -> &'static str {
        match self {
            PeriodicityAdjustment::Actual => "ACTUAL",
            PeriodicityAdjustment::Calendar => "CALENDAR",
            PeriodicityAdjustment::Fiscal => "FISCAL",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PeriodicitySelection {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    SemiAnnually,
    Yearly,
}

impl PeriodicitySelection {
    pub fn as_str(self) -> &'static str {
        match self {
            PeriodicitySelection::Daily => "DAILY",
            PeriodicitySelection::Weekly => "WEEKLY",
            PeriodicitySelection::Monthly => "MONTHLY",
            PeriodicitySelection::Quarterly => "QUARTERLY",
            PeriodicitySelection::SemiAnnually => "SEMIANNUALLY",
            PeriodicitySelection::Yearly => "YEARLY",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_request() -> Result<(), Error> {
        let mut session: SessionSync = SessionOptions::default()
            .with_server_host("localhost")?
            .with_server_port(8194)?
            .sync();

        Ok(())
    }
}
