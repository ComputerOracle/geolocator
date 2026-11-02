use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub status: u16,
    pub details: Option<String>,
}

#[derive(Debug)]
pub enum AppError {
    InvalidCoordinates(String),
    InvalidRssi(String),
    DeviceNotFound(String),
    BluetoothUnavailable(String),
    LockError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::InvalidCoordinates(msg) => write!(f, "Invalid coordinates: {msg}"),
            AppError::InvalidRssi(msg) => write!(f, "Invalid RSSI value: {msg}"),
            AppError::DeviceNotFound(id) => write!(f, "Device not found: {id}"),
            AppError::BluetoothUnavailable(msg) => write!(f, "Bluetooth adapter unavailable: {msg}"),
            AppError::LockError(msg) => write!(f, "Internal lock error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::InvalidCoordinates(_) | AppError::InvalidRssi(_) => StatusCode::BAD_REQUEST,
            AppError::DeviceNotFound(_) => StatusCode::NOT_FOUND,
            AppError::BluetoothUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::LockError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        HttpResponse::build(status).json(ErrorResponse {
            error: status.canonical_reason().unwrap_or("Error").to_string(),
            status: status.as_u16(),
            details: Some(self.to_string()),
        })
    }
}
