use crate::bindings::*;
use crate::{correlation_id::CorrelationId, element::Element, event::Event, name::Name};
use std::ffi::CStr;
use std::marker::PhantomData;

pub struct Message<'a> {
    pub(crate) ptr: *mut blpapi_Message_t,
    pub(crate) _phantom: PhantomData<&'a Event>,
    pub(crate) elements: *mut blpapi_Element_t,
}

impl<'a> Message<'a> {
    pub fn topic_name(&self) -> String {
        unsafe {
            let name: *const i8 = blpapi_Message_topicName(self.ptr);

            CStr::from_ptr(name).to_string_lossy().into_owned()
        }
    }

    pub fn type_string(&self) -> String {
        unsafe {
            let name: *const i8 = blpapi_Message_typeString(self.ptr);

            CStr::from_ptr(name).to_string_lossy().into_owned()
        }
    }

    pub fn message_type(&self) -> Name {
        unsafe {
            let ptr: *mut blpapi_Name = blpapi_Message_messageType(self.ptr);

            Name(ptr)
        }
    }

    pub fn num_correlation_ids(&self) -> usize {
        unsafe { blpapi_Message_numCorrelationIds(self.ptr) as usize }
    }

    pub fn correlation_id(&self, index: usize) -> Option<CorrelationId> {
        if index > self.num_correlation_ids() {
            None
        } else {
            unsafe {
                let ptr: blpapi_CorrelationId_t_ = blpapi_Message_correlationId(self.ptr, index);

                Some(CorrelationId(ptr))
            }
        }
    }

    pub fn element(&self) -> Element {
        Element { ptr: self.elements }
    }
}
