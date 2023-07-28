use std::marker::PhantomData;

pub struct Enum<T>(PhantomData<T>);

#[cfg(not(nightly_column_names))]
pub struct EnumVariant<const NAME: usize>(());
#[cfg(nightly_column_names)]
pub struct EnumVariant<const NAME: &'static str>(());
