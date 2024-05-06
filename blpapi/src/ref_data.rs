use crate::element::Element;

pub trait RefData: Default {
    const FIELDS: &'static [&'static str];

    fn on_field(&mut self, field: &str, element: &Element);
}
