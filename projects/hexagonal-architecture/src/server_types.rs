use super::traits::*;
use leptos::config::LeptosOptions;
use thiserror::Error;

#[derive(Clone)]
pub struct ServerState<Handler: HandlerTrait> {
    pub handler: Handler,
    pub leptos_options: LeptosOptions,
}

#[cfg(feature = "config_1")]
pub type HandlerStructAlias = HandlerStruct<
    SubDomainStruct1<ExternalService1_1, ExternalService2_1>,
    SubDomainStruct2<ExternalService1_1>,
>;
#[cfg(not(feature = "config_1"))]
pub type HandlerStructAlias = HandlerStruct<
    SubDomainStruct1<ExternalService1_2, ExternalService2_2>,
    SubDomainStruct2<ExternalService1_2>,
>;

#[derive(Clone, Default)]
pub struct HandlerStruct<SubDomain1: SubDomainTrait1, SubDomain2: SubDomainTrait2> {
    pub sub_domain_1: SubDomain1,
    pub sub_domain_2: SubDomain2,
}
#[derive(Clone, Default)]
pub struct SubDomainStruct1<
    ExternalService1: ExternalServiceTrait1,
    ExternalService2: ExternalServiceTrait2,
> {
    pub external_service_1: ExternalService1,
    pub external_service_2: ExternalService2,
}

#[derive(Clone, Default)]
pub struct SubDomainStruct2<ExternalService1: ExternalServiceTrait1> {
    pub external_service_1: ExternalService1,
}

#[derive(Clone, Default)]
pub struct ExternalService1_1;
#[derive(Clone, Default)]
pub struct ExternalService1_2;
#[derive(Clone, Default)]
pub struct ExternalService2_1;
#[derive(Clone, Default)]
pub struct ExternalService2_2;
#[derive(Clone, Default)]
pub struct ExternalService1;

#[derive(Clone, PartialEq, Debug)]
pub struct DomainData;
#[derive(Clone, PartialEq, Debug)]
pub struct DomainData2;
#[derive(Clone, PartialEq, Debug)]
pub struct DomainData3;
#[derive(Clone, PartialEq, Debug)]
pub struct SubDomain1Data;
#[derive(Clone, PartialEq, Debug)]
pub struct SubDomain2Data;
#[derive(Clone)]
pub struct ExternalService1Data;
#[derive(Clone)]
pub struct ExternalService2Data;

#[derive(Clone, Error, Debug)]
pub enum DomainError {
    #[error("Underlying Subdomain 1 Error")]
    SubDomain1Error(#[from] SubDomain1Error),
    #[error("Underlying Subdomain 2 Error")]
    SubDomain2Error(#[from] SubDomain2Error),
}

#[derive(Clone, Error, Debug)]
pub enum SubDomain1Error {
    #[error("Sub Domain 1 Error")]
    SubDomain1Error,
    #[error("Underlying Service 1")]
    ExternalService1Error(#[from] ExternalService1Error),
    #[error("Underlying Service 2")]
    ExternalService2Error(#[from] ExternalService2Error),
}
#[derive(Clone, Error, Debug)]
pub enum SubDomain2Error {
    #[error("Sub Domain 2 Error")]
    SubDomain2Error,
    #[error("Underlying Service 1")]
    ExternalService1Error(#[from] ExternalService1Error),
}

#[derive(Clone, Error, Debug)]
pub enum ExternalService1Error {
    #[error("Service 1 Error")]
    Error,
}

#[derive(Clone, Error, Debug)]
pub enum ExternalService2Error {
    #[error("Service 2 Error")]
    Error,
}
