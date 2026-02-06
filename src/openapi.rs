use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::Modify;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "EDrota API",
        version = "1.0.0",
        description = "Backend API for EDrota shift management system",
        contact(
            name = "API Support",
            email = "support@edrota.com"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
    ),
    paths(
        // Health
        crate::handlers::health::health_check,

        // Auth
        crate::handlers::auth_handler::get_me,
        crate::handlers::auth_handler::verify_pin,

        // Users
        crate::handlers::users_handler::get_users,
        crate::handlers::users_handler::get_user,
        crate::handlers::users_handler::get_substantive_users,
        crate::handlers::users_handler::get_staff_list,
        crate::handlers::users_handler::update_own_profile,
        crate::handlers::users_handler::change_own_pin,
        crate::handlers::users_handler::update_user_profile,
        crate::handlers::users_handler::reset_user_pin,

        // References
        crate::handlers::references_handler::get_time_off_categories,

        // Comments
        crate::handlers::comments_handler::get_comments,

        // Audit
        crate::handlers::audit_handler::get_audit,

        // Shifts
        crate::handlers::shifts_handler::get_shifts_for_month,
        crate::handlers::shifts_handler::get_shifts_for_date,
        crate::handlers::shifts_handler::get_shifts_for_range,
        crate::handlers::shifts_handler::create_shift,
        crate::handlers::shifts_handler::update_shift,
        crate::handlers::shifts_handler::delete_shift,

        // Templates
        crate::handlers::templates_handler::get_templates,
        crate::handlers::templates_handler::create_template,
        crate::handlers::templates_handler::update_template,
        crate::handlers::templates_handler::delete_template,

        // Diary
        crate::handlers::diary_handler::get_diary,
        crate::handlers::diary_handler::create_diary_entry,
        crate::handlers::diary_handler::delete_diary_entry,

        // Job Plans
        crate::handlers::job_plans_handler::get_job_plans,
        crate::handlers::job_plans_handler::create_job_plan,
        crate::handlers::job_plans_handler::update_job_plan,
        crate::handlers::job_plans_handler::delete_job_plan,
        crate::handlers::job_plans_handler::terminate_job_plan,

        // User Roles
        crate::handlers::user_roles_handler::get_user_roles,
        crate::handlers::user_roles_handler::create_user_role,
        crate::handlers::user_roles_handler::update_user_role,
        crate::handlers::user_roles_handler::delete_user_role,

        // Roles
        crate::handlers::roles_handler::get_roles,
        crate::handlers::roles_handler::create_role,
        crate::handlers::roles_handler::update_role,
        crate::handlers::roles_handler::delete_role,

        // Workplaces
        crate::handlers::workplaces_handler::get_workplaces,
        crate::handlers::workplaces_handler::create_workplace,
        crate::handlers::workplaces_handler::update_workplace,
        crate::handlers::workplaces_handler::delete_workplace,

        // Marketplace
        crate::handlers::marketplace_handler::get_open_requests,
        crate::handlers::marketplace_handler::get_my_requests,
        crate::handlers::marketplace_handler::get_incoming_requests,
        crate::handlers::marketplace_handler::get_approval_requests,
        crate::handlers::marketplace_handler::get_dashboard,
        crate::handlers::marketplace_handler::get_swappable_shifts,
        crate::handlers::marketplace_handler::create_shift_request,
        crate::handlers::marketplace_handler::accept_shift_request,
        crate::handlers::marketplace_handler::respond_to_proposal,
        crate::handlers::marketplace_handler::admin_decision,
        crate::handlers::marketplace_handler::cancel_shift_request,
    ),
    components(
        schemas(
            // Core models
            crate::models::User,
            crate::models::UserRole,
            crate::models::Role,
            crate::models::Workplace,
            crate::models::Shift,
            crate::models::ShiftTemplate,
            crate::models::DiaryEntry,
            crate::models::JobPlan,
            crate::models::ShiftRequest,
            crate::models::ShiftRequestWithDetails,
            crate::models::TimeOffCategory,
            crate::models::AuditEntry,
            crate::models::COD,
            crate::models::StaffFilterOption,

            // Input models
            crate::models::CreateShiftInput,
            crate::models::UpdateShiftInput,
            crate::models::ShiftMutationResponse,
            crate::models::CreateDiaryInput,
            crate::models::DiaryMutationResponse,
            crate::models::CreateJobPlanInput,
            crate::models::UpdateJobPlanInput,
            crate::models::JobPlanMutationResponse,
            crate::models::CreateTemplateInput,
            crate::models::UpdateTemplateInput,
            crate::models::TemplateMutationResponse,
            crate::models::UpdateOwnProfileInput,
            crate::models::ChangeOwnPinInput,
            crate::models::UpdateUserProfileInput,
            crate::models::PinResponse,
            crate::models::CreateUserRoleInput,
            crate::models::UpdateUserRoleInput,
            crate::models::UserRoleMutationResponse,
            crate::models::CreateRoleInput,
            crate::models::UpdateRoleInput,
            crate::models::RoleMutationResponse,
            crate::models::CreateWorkplaceInput,
            crate::models::UpdateWorkplaceInput,
            crate::models::WorkplaceMutationResponse,
            crate::models::CreateShiftRequestInput,
            crate::models::AcceptRequestInput,
            crate::models::RespondToProposalInput,
            crate::models::AdminDecisionInput,
            crate::models::MarketplaceMutationResponse,

            // Auth types
            crate::handlers::auth_handler::VerifyPinRequest,
            crate::handlers::auth_handler::VerifyPinResponse,
        )
    ),
    tags(
        (name = "health", description = "Health check"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User management"),
        (name = "shifts", description = "Shift management"),
        (name = "templates", description = "Shift template management"),
        (name = "diary", description = "Diary entry management"),
        (name = "job-plans", description = "Job plan management"),
        (name = "user-roles", description = "User role assignment management"),
        (name = "roles", description = "Role management"),
        (name = "workplaces", description = "Workplace management"),
        (name = "marketplace", description = "Shift swap marketplace"),
        (name = "references", description = "Reference data"),
        (name = "comments", description = "Comments and COD"),
        (name = "audit", description = "Audit trail"),
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "cookie_auth",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("__session"))),
            )
        }
    }
}
