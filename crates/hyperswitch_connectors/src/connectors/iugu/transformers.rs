use std::fmt::Debug;

use common_enums::{enums, AttemptStatus, RefundStatus};
use common_utils::types::StringMinorUnit;
use masking::Secret;
use serde::{Deserialize, Serialize};

use hyperswitch_domain_models::{
    router_data::RouterData,
    router_flow_types::{Authorize, Capture, Execute, PSync, RSync, Void},
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCaptureData, PaymentsCancelData, PaymentsSyncData,
    },
    router_response_types::{
        PaymentsResponseData, RefundsResponseData as RefundsResponseDataType,
        RedirectForm, ResponseId,
    },
    payment_method_data::PaymentMethodData,
};

use hyperswitch_interfaces::{
    api::{
        self, ConnectorCommon, ConnectorCommonExt, ConnectorIntegration, ConnectorSpecifications,
        ConnectorValidation,
    },
    configs::Connectors,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::{self, Response},
    webhooks,
};

use common_utils::ext_traits::OptionExt;

use crate::types::{
    api as api_types, storage::enums as storage_enums, Connector, ConnectorData,
    TransformersData,
};

use crate::types::{RefundsRouterData, RefundSyncRouterData, ResponseRouterData, RefundsResponseRouterData};
use hyperswitch_domain_models::router_response_types::RefundsResponseData as RefundsResponseDataType;
use hyperswitch_domain_models::router_request_types::ResponseId;

//TODO: Fill the struct with respective fields
pub struct IuguRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for IuguRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct IuguPaymentsRequest {
    amount: StringMinorUnit,
    card: IuguCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct IuguCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&IuguRouterData<&PaymentsAuthorizeRouterData>> for IuguPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &IuguRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = IuguCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct IuguAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for IuguAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IuguPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<IuguPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: IuguPaymentStatus) -> Self {
        match item {
            IuguPaymentStatus::Succeeded => Self::Charged,
            IuguPaymentStatus::Failed => Self::Failure,
            IuguPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IuguPaymentsResponse {
    status: IuguPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, IuguPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, IuguPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct IuguRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&IuguRouterData<&RefundsRouterData<F>>> for IuguRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &IuguRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct IuguErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

// --- IUGU REQUEST STRUCTS ---
#[derive(Debug, Serialize, Default)]
pub struct IuguChargeRequest {
    pub token: Option<String>,
    pub customer_payment_method_id: Option<String>,
    pub email: String,
    pub order_id: Option<String>,
    pub payer: Option<IuguPayerInfo>,
    pub invoice_id: Option<String>,
    pub items: Option<Vec<IuguItem>>,
    // pub splits: Option<Vec<IuguSplitRule>>, // charge não aceita split direto, só invoice
}

#[derive(Debug, Serialize, Default)]
pub struct IuguInvoiceRequest {
    pub email: String,
    pub due_date: Option<String>,
    pub payable_with: Vec<String>,
    pub items: Vec<IuguItem>,
    pub payer: Option<IuguPayerInfo>,
    pub ensure_workday_due_date: Option<bool>,
    pub splits: Option<Vec<IuguSplitRule>>,
    pub order_id: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct IuguItem {
    pub description: String,
    pub quantity: u32,
    pub price_cents: u64,
}

#[derive(Debug, Serialize, Default)]
pub struct IuguPayerInfo {
    pub name: String,
    pub cpf_cnpj: String,
    pub email: Option<String>,
    pub phone_prefix: Option<String>,
    pub phone: Option<String>,
    pub address: Option<IuguAddress>,
}

#[derive(Debug, Serialize, Default)]
pub struct IuguAddress {
    pub street: Option<String>,
    pub number: Option<String>,
    pub district: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zip_code: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct IuguSplitRule {
    pub recipient_account_id: String,
    pub percentage: Option<f64>,
    pub amount_cents: Option<u64>,
    pub liable: Option<bool>,
    pub charge_fee: Option<bool>,
}

// --- IUGU STATUS ENUM ---
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IuguInvoiceStatus {
    Pending,
    Paid,
    Canceled,
    Expired,
    Refunded,
    InAnalysis,
    PartiallyPaid,
}

impl Default for IuguInvoiceStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl From<IuguInvoiceStatus> for enums::AttemptStatus {
    fn from(item: IuguInvoiceStatus) -> Self {
        match item {
            IuguInvoiceStatus::Paid => Self::Charged,
            IuguInvoiceStatus::Refunded => Self::Charged,
            IuguInvoiceStatus::Pending | IuguInvoiceStatus::InAnalysis | IuguInvoiceStatus::PartiallyPaid => Self::Pending,
            IuguInvoiceStatus::Canceled => Self::Voided,
            IuguInvoiceStatus::Expired => Self::Failure,
        }
    }
}

// --- IUGU RESPONSE STRUCTS ---
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IuguInvoiceResponse {
    pub id: String,
    pub status: IuguInvoiceStatus,
    pub secure_url: Option<String>,
    pub pdf: Option<String>,
    pub pix: Option<IuguPixDetails>,
    pub error: Option<String>,
    pub errors: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IuguPixDetails {
    pub qrcode: Option<String>,
    pub qrcode_text: Option<String>,
    pub expires_at: Option<String>,
}

// --- TRYFROM IMPLEMENTATIONS ---
use crate::utils::PaymentsAuthorizeRequestData;
use hyperswitch_domain_models::router_request_types::PaymentsAuthorizeRouterData;

impl TryFrom<&PaymentsAuthorizeRouterData> for IuguChargeRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let email = item.request.email.clone().ok_or_else(|| errors::ConnectorError::MissingRequiredField { field_name: "email" })?;
        let order_id = item.connector_request_reference_id.clone();
        let payer = None; // TODO: extrair payer se necessário
        let token = None; // TODO: extrair token se necessário
        let method_id = None; // TODO: extrair method_id se necessário
        Ok(IuguChargeRequest {
            token,
            customer_payment_method_id: method_id,
            email: email.to_string(),
            order_id: Some(order_id),
            payer,
            invoice_id: None,
            items: None,
        })
    }
}

impl TryFrom<&PaymentsAuthorizeRouterData> for IuguInvoiceRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let email = item.request.email.clone().ok_or_else(|| errors::ConnectorError::MissingRequiredField { field_name: "email" })?;
        let order_id = item.connector_request_reference_id.clone();
        let payer = None; // TODO: extrair payer se necessário
        let due_date = None; // TODO: calcular se boleto
        let payable_with = vec!["bank_slip".to_string()]; // TODO: ajustar para pix/cartão
        let items = vec![]; // TODO: popular itens
        Ok(IuguInvoiceRequest {
            email: email.to_string(),
            due_date,
            payable_with,
            items,
            payer,
            ensure_workday_due_date: None,
            splits: None,
            order_id: Some(order_id),
        })
    }
}

// --- TRYFROM IMPLEMENTATIONS FOR IUGU INVOICE RESPONSE ---
impl TryFrom<ResponseRouterData<Authorize, IuguInvoiceResponse, PaymentsAuthorizeData, PaymentsResponseData>>
    for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<Authorize, IuguInvoiceResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::from(item.response.status.clone());
        let resource_id = ResponseId::ConnectorTransactionId(item.response.id.clone());
        let redirection_data = item.response.secure_url.as_ref().map(|url| {
            Box::new(Some(RedirectForm::from((url, services::Method::Get))))
        }).unwrap_or(Box::new(None));
        let payments_response = PaymentsResponseData::TransactionResponse {
            resource_id,
            redirection_data,
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        };
        Ok(RouterData {
            response: Ok(payments_response),
            status,
            ..item.data
        })
    }
}

impl TryFrom<ResponseRouterData<PSync, IuguInvoiceResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<PSync, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<PSync, IuguInvoiceResponse, PaymentsSyncData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::from(item.response.status.clone());
        let resource_id = ResponseId::ConnectorTransactionId(item.response.id.clone());
        let payments_response = PaymentsResponseData::TransactionResponse {
            resource_id,
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        };
        Ok(RouterData {
            response: Ok(payments_response),
            status,
            ..item.data
        })
    }
}

impl TryFrom<ResponseRouterData<Capture, IuguInvoiceResponse, PaymentsCaptureData, PaymentsResponseData>>
    for RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<Capture, IuguInvoiceResponse, PaymentsCaptureData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::from(item.response.status.clone());
        let resource_id = ResponseId::ConnectorTransactionId(item.response.id.clone());
        let payments_response = PaymentsResponseData::TransactionResponse {
            resource_id,
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        };
        Ok(RouterData {
            response: Ok(payments_response),
            status,
            ..item.data
        })
    }
}

impl TryFrom<ResponseRouterData<Void, IuguInvoiceResponse, PaymentsCancelData, PaymentsResponseData>>
    for RouterData<Void, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<Void, IuguInvoiceResponse, PaymentsCancelData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::from(item.response.status.clone());
        let resource_id = ResponseId::ConnectorTransactionId(item.response.id.clone());
        let payments_response = PaymentsResponseData::TransactionResponse {
            resource_id,
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        };
        Ok(RouterData {
            response: Ok(payments_response),
            status,
            ..item.data
        })
    }
}
