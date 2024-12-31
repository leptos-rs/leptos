use super::server_types::*;

pub fn config() -> HandlerStructAlias {
    cfg_if::cfg_if! {
                if #[cfg(feature="config_1")] {
                    fn server_handler_config_1() -> HandlerStruct<
            SubDomainStruct1<ExternalService1_1, ExternalService2_1>,
            SubDomainStruct2<ExternalService1_1>,
        > {
            HandlerStruct::default()
        }
                    server_handler_config_1()
                } else {
                    fn server_handler_config_2() -> HandlerStruct<
        SubDomainStruct1<ExternalService1_2, ExternalService2_2>,
        SubDomainStruct2<ExternalService1_2>,
    > {
        HandlerStruct::new()
    }
                    server_handler_config_2()
                }
            }
}
