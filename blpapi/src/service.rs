use crate::{request::Request, Error};
use blpapi_sys::*;
use std::ffi::CStr;

pub struct Service(pub(crate) *mut blpapi_Service_t);

impl Service {
    pub fn name(&self) -> String {
        let name: &CStr = unsafe { CStr::from_ptr(blpapi_Service_name(self.0)) };
        name.to_string_lossy().into_owned()
    }

    pub fn create_request(&self, operation: &str) -> Result<Request, Error> {
        Request::new(self, operation)
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        unsafe { blpapi_Service_release(self.0) }
    }
}
