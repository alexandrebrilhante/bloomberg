use crate::{
    element::{Element, SetValue},
    name::Name,
    service::Service,
    Error,
};
use blpapi_sys::*;
use std::ffi::CString;

pub struct Request {
    pub(crate) ptr: *mut blpapi_Request_t,
    elements: *mut blpapi_Element_t,
}

impl Request {
    pub fn new(service: &Service, operation: &str) -> Result<Self, Error> {
        let operation: CString = CString::new(operation).unwrap();

        unsafe {
            let mut ptr: *mut blpapi_Request = std::ptr::null_mut();

            let ref_ptr: *mut *mut blpapi_Request = &mut ptr as *mut _;

            let res: i32 = blpapi_Service_createRequest(service.0, ref_ptr, operation.as_ptr());

            Error::check(res)?;

            let elements: *mut blpapi_Element = blpapi_Request_elements(ptr);

            Ok(Request { ptr, elements })
        }
    }

    pub fn element(&self) -> Element {
        Element { ptr: self.elements }
    }

    pub fn append<V: SetValue>(&mut self, name: &str, value: V) -> Result<(), Error> {
        let mut element: Element = self
            .element()
            .get_element(name)
            .ok_or_else(|| Error::NotFound(name.to_owned()))?;

        element.append(value)
    }

    pub fn append_named<V: SetValue>(&mut self, name: &Name, value: V) -> Result<(), Error> {
        self.element()
            .get_named_element(name)
            .ok_or_else(|| Error::NotFound(name.to_string()))?
            .append(value)
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        unsafe { blpapi_Request_destroy(self.ptr) }
    }
}
