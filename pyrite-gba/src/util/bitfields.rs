pub trait FieldConvert<Out> {
    fn convert(self) -> Out;
}

macro_rules! impl_enum_bitfield_conv {
    ($EnumName:ident: $FieldType:ty, $($Variant:ident = $Value:expr,)+) => {
        impl $crate::util::bitfields::FieldConvert<$FieldType> for $EnumName {
            #[inline]
            fn convert(self) -> $FieldType {
                match self {
                    $(
                        $EnumName::$Variant => $Value,
                    )+
                }
            }
        }

        impl $crate::util::bitfields::FieldConvert<$EnumName> for $FieldType {
            #[inline]
            fn convert(self) -> $EnumName {
                match self {
                    $(
                        $Value => $EnumName::$Variant,
                    )+
                    _ => unreachable!("bad enum bitfield conversion"),
                }
            }
        }
    };
}

macro_rules! as_conversion {
    ($From:ty, $To:ty) => {
        impl FieldConvert<$To> for $From {
            fn convert(self) -> $To {
                self as $To
            }
        }
    };
}

macro_rules! impl_unit_struct_field_convert {
    ($StructType:ident, $ConversionType:ty) => {
        impl crate::util::bitfields::FieldConvert<$ConversionType> for $StructType {
            fn convert(self) -> $ConversionType {
                self.0 as $ConversionType
            }
        }

        impl crate::util::bitfields::FieldConvert<$StructType> for $ConversionType {
            fn convert(self) -> $StructType {
                $StructType(self as _)
            }
        }
    };
}

impl FieldConvert<bool> for u8 {
    fn convert(self) -> bool {
        self != 0
    }
}
impl FieldConvert<bool> for u16 {
    fn convert(self) -> bool {
        self != 0
    }
}
impl FieldConvert<bool> for u32 {
    fn convert(self) -> bool {
        self != 0
    }
}
impl FieldConvert<bool> for u64 {
    fn convert(self) -> bool {
        self != 0
    }
}

as_conversion!(bool, u8);
as_conversion!(bool, u16);
as_conversion!(bool, u32);
as_conversion!(bool, u64);

as_conversion!(u8, u8);
as_conversion!(u8, u16);
as_conversion!(u8, u32);
as_conversion!(u8, u64);

as_conversion!(u16, u8);
as_conversion!(u16, u16);
as_conversion!(u16, u32);
as_conversion!(u16, u64);

as_conversion!(u32, u8);
as_conversion!(u32, u16);
as_conversion!(u32, u32);
as_conversion!(u32, u64);

as_conversion!(u64, u8);
as_conversion!(u64, u16);
as_conversion!(u64, u32);
as_conversion!(u64, u64);

#[macro_export]
macro_rules! bitfields {
    ($TypeName:ident : $ValueType:ty {
        $(
            $FieldGet:ident, $FieldSet:ident: $FieldType:ty = [$FieldStart:expr, $FieldEnd:expr],
        )*
    }) => {
        #[derive(Debug, Copy, Clone)]
        pub struct $TypeName {
            pub value: $ValueType,
        }

        impl $TypeName {
            pub const fn wrap(value: $ValueType) -> $TypeName {
                $TypeName { value }
            }

            $(
                pub fn $FieldGet(&self) -> $FieldType {
                    crate::util::bitfields::FieldConvert::<$FieldType>::convert((self.value >> $FieldStart) & ((1<<($FieldEnd-$FieldStart+1)) - 1))
                }

                pub fn $FieldSet(&mut self, value: $FieldType) {
                    let value = crate::util::bitfields::FieldConvert::<$ValueType>::convert(value);
                    self.value = (self.value & !(((1<<($FieldEnd-$FieldStart+1)) - 1) << $FieldStart)) |
                        ((value & ((1<<($FieldEnd-$FieldStart+1)) - 1)) << $FieldStart);
                }
            )*
        }

        impl Default for $TypeName {
            fn default() -> $TypeName {
                Self::wrap(0)
            }
        }
    }
}
