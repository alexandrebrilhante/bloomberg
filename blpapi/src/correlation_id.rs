use blpapi_sys::*;
use std::os::raw::c_uint;

const DEFAULT_CLASS_ID: c_uint = 0;

pub struct CorrelationId(pub(crate) blpapi_CorrelationId_t);
impl CorrelationId {
    pub fn new_u64(value: u64) -> Self {
        let size: u32 = std::mem::size_of::<blpapi_CorrelationId_t>() as c_uint;
        let value_type: u32 = BLPAPI_CORRELATION_TYPE_INT;
        let class_id: u32 = DEFAULT_CLASS_ID;
        let reserved: u32 = 0;

        let _bitfield_1: __BindgenBitfieldUnit<[u8; 4], u16> =
            blpapi_CorrelationId_t_::new_bitfield_1(size, value_type, class_id, reserved);

        let value: blpapi_CorrelationId_t___bindgen_ty_1 =
            blpapi_CorrelationId_t___bindgen_ty_1 { intValue: value };

        let inner: blpapi_CorrelationId_t_ = blpapi_CorrelationId_t_ { value, _bitfield_1 };

        CorrelationId(inner)
    }
}

#[test]
fn correlation_u64() {
    let id: CorrelationId = CorrelationId::new_u64(1);

    assert_eq!(unsafe { id.0.value.intValue }, 1);
}
