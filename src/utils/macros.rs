#[macro_export]
macro_rules! define_struct_with_defaults {
    ($config_struct:ident, $struct_name:ident {
        $( $field_name:ident : $field_type:ty = $default:expr ),* $(,)?
    }) => {
        pub struct $config_struct {
            $( pub $field_name: Option<$field_type>, )*
        }

        pub struct $struct_name {
            $( pub $field_name: $field_type, )*
        }

        impl Default for $config_struct {
            fn default() -> Self {
                Self {
                    $( $field_name: Some($default), )*
                }
            }
        }

        impl $config_struct {
            pub fn build(self) -> $struct_name {
                $struct_name {
                    $( $field_name: self.$field_name.unwrap_or($default), )*
                }
            }
        }
    };
}