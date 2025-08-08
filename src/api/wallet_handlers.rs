use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use tracing::{error, info};
use uuid::Uuid;

use super::{ApiResponse, ApiState};
use crate::wallets::{CreateWalletRequest, Pagination, UpdateWalletRequest, WalletFilters};

#[derive(Debug, Deserialize)]
pub struct WalletListQuery {
    pub chain: Option<String>,
    pub wallet_type: Option<String>,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
    pub name_contains: Option<String>,
    pub needs_sync: Option<bool>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub async fn create_wallet_handler(
    State(state): State<ApiState>,
    Json(payload): Json<CreateWalletRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    info!("Create wallet requested: {}", payload.name);

    match state.wallet_manager.create_wallet(payload).await {
        Ok(wallet) => Ok(Json(ApiResponse::success(serde_json::json!({
            "id": wallet.id,
            "name": wallet.name,
            "chain": wallet.chain,
            "type": wallet.wallet_type,
            "status": wallet.status,
        })))),
        Err(e) => {
            error!("Failed to create wallet: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(e.to_string())),
            ))
        }
    }
}

pub async fn list_wallets_handler(
    State(state): State<ApiState>,
    Query(q): Query<WalletListQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let filters = WalletFilters {
        chain: None,
        wallet_type: None,
        status: None,
        tags: q.tags.clone(),
        name_contains: q.name_contains.clone(),
        needs_sync: q.needs_sync,
    };

    let pagination = Pagination {
        page: q.page,
        limit: q.limit,
    };

    match state
        .wallet_manager
        .list_wallets(Some(filters), Some(pagination))
        .await
    {
        Ok(resp) => Ok(Json(ApiResponse::success(serde_json::json!({
            "wallets": resp.wallets,
            "total": resp.total,
            "page": resp.page,
            "limit": resp.limit,
            "total_pages": resp.total_pages
        })))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )),
    }
}

pub async fn get_wallet_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Invalid UUID".into())),
        )
    })?;

    match state.wallet_manager.get_wallet(id).await {
        Ok(Some(wallet)) => Ok(Json(ApiResponse::success(serde_json::json!(wallet)))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("Wallet not found".into())),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )),
    }
}

pub async fn update_wallet_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateWalletRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Invalid UUID".into())),
        )
    })?;

    match state.wallet_manager.update_wallet(id, payload).await {
        Ok(wallet) => Ok(Json(ApiResponse::success(serde_json::json!(wallet)))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )),
    }
}

pub async fn delete_wallet_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Invalid UUID".into())),
        )
    })?;

    match state.wallet_manager.delete_wallet(id).await {
        Ok(_) => Ok(Json(ApiResponse::success(
            serde_json::json!({ "deleted": true }),
        ))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )),
    }
}

pub async fn sync_wallet_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Invalid UUID".into())),
        )
    })?;

    let wallet = state
        .wallet_manager
        .get_wallet(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(e.to_string())),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("Wallet not found".into())),
        ))?;

    match state.wallet_sync.sync_wallet(&wallet).await {
        Ok(stats) => Ok(Json(ApiResponse::success(serde_json::json!(stats)))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(e.to_string())),
        )),
    }
}

pub async fn get_wallet_balances_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    let id = Uuid::parse_str(&id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Invalid UUID".into())),
        )
    })?;

    let wallet = state
        .wallet_manager
        .get_wallet(id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(e.to_string())),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("Wallet not found".into())),
        ))?;

    let balances: Vec<_> = wallet
        .addresses
        .iter()
        .filter_map(|a| a.balance.clone())
        .collect();
    Ok(Json(ApiResponse::success(
        serde_json::json!({ "balances": balances }),
    )))
}

pub async fn get_wallet_sync_stats_handler(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Placeholder until persisted sync stats
    Ok(Json(ApiResponse::success(serde_json::json!({
        "wallet_id": id,
        "last_sync": null,
        "addresses_synced": 0
    }))))
}
