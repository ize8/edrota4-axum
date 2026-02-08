use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;


/// Input for creating a shift swap request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateShiftRequestInput {
    pub shift_id: Uuid,
    #[serde(rename = "type")]
    pub request_type: String, // "SWAP" or "GIVE_AWAY"
    pub target_user_id: Option<i32>,
    pub target_shift_id: Option<Uuid>,
    pub notes: Option<String>,
    #[serde(rename = "confirmedRequesterId")]
    pub confirmed_requester_id: Option<i32>, // For generic accounts - PIN-verified user ID
}

/// Input for accepting/claiming an open request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcceptRequestInput {
    pub target_shift_id: Option<Uuid>, // Optional - only needed if proposing a swap
    #[serde(rename = "confirmedCandidateId")]
    pub confirmed_candidate_id: Option<i32>, // For generic accounts - PIN-verified user ID
}

/// Input for responding to a proposed swap (approve or reject by target user)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RespondToProposalInput {
    pub accept: bool, // true = accept, false = reject
    #[serde(rename = "confirmedResponderId")]
    pub confirmed_responder_id: Option<i32>, // For generic accounts - PIN-verified user ID
}

/// Input for admin approval decision
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminDecisionInput {
    pub approve: bool, // true = approve, false = reject
    pub notes: Option<String>,
}

/// Response for marketplace mutations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarketplaceMutationResponse {
    pub success: bool,
    pub message: Option<String>,
}
