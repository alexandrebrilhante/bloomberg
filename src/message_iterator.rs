use crate::bindings::*;
use crate::{event::Event, message::Message};
use std::marker::PhantomData;
use std::ptr;

pub struct MessageIterator<'a> {
    pub(crate) ptr: *mut blpapi_MessageIterator_t,
    _phantom: PhantomData<&'a Event>,
}

impl<'a> MessageIterator<'a> {
    pub fn new(event: &'a Event) -> Self {
        unsafe {
            let ptr: *mut blpapi_MessageIterator = blpapi_MessageIterator_create(event.0);

            MessageIterator {
                ptr,
                _phantom: PhantomData,
            }
        }
    }
}

impl<'a> Drop for MessageIterator<'a> {
    fn drop(&mut self) {
        unsafe { blpapi_MessageIterator_destroy(self.ptr) }
    }
}

impl<'a> Iterator for MessageIterator<'a> {
    type Item = Message<'a>;

    fn next(&mut self) -> Option<Message<'a>> {
        unsafe {
            let mut ptr: *mut blpapi_Message = ptr::null_mut();
            let res: i32 = blpapi_MessageIterator_next(self.ptr, &mut ptr as *mut _);

            if res == 0 {
                let elements: *mut blpapi_Element = blpapi_Message_elements(ptr);

                Some(Message {
                    ptr,
                    _phantom: PhantomData,
                    elements,
                })
            } else {
                None
            }
        }
    }
}
