//! Human-readable names for raw SDK enum values carried in [`crate::Event`].
//! Values verified against `export_h/ToolSuiteProxyDef.h` of
//! zoom-plugin-sdk-windows-7.2.0.3001. Re-check when upgrading the SDK.

fn pick(names: &[&'static str], code: i32) -> String {
    usize::try_from(code)
        .ok()
        .and_then(|i| names.get(i))
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("UNKNOWN({code})"))
}

/// `ZMToolSuiteProxyAuthResult`
pub fn auth_result_name(code: i32) -> String {
    pick(
        &["AUTH_RESULT_SUCCESS", "AUTH_RESULT_REQUEST_ERROR", "AUTH_RESULT_VERIFY_USER_FAILED", "AUTH_RESULT_FAILED"],
        code,
    )
}
pub const AUTH_RESULT_SUCCESS: i32 = 0;

/// `ZMToolSuiteIPCConnectStatus`
pub fn ipc_status_name(code: i32) -> String {
    pick(&["IPC_STATUS_IDLE", "IPC_STATUS_CONNECT_TIMEOUT", "IPC_STATUS_CONNECTED", "IPC_STATUS_DISCONNECTED"], code)
}
pub const IPC_STATUS_CONNECTED: i32 = 2;

/// `ZMToolSuiteMeetingStatus`
pub fn meeting_status_name(code: i32) -> String {
    pick(
        &[
            "MEETING_STATUS_IDLE",
            "MEETING_STATUS_CONNECTING",
            "MEETING_STATUS_WAITING_FOR_HOST",
            "MEETING_STATUS_INMEETING",
            "MEETING_STATUS_DISCONNECTING",
            "MEETING_STATUS_RECONNECTING",
            "MEETING_STATUS_FAILED",
            "MEETING_STATUS_ENDED",
            "MEETING_STATUS_LOCKED",
            "MEETING_STATUS_UNLOCKED",
            "MEETING_STATUS_IN_WAITING_ROOM",
            "MEETING_STATUS_WEBINAR_PROMOTE",
            "MEETING_STATUS_WEBINAR_DEPROMOTE",
            "MEETING_STATUS_BEFORE_SUBCONF_CREATE",
            "MEETING_STATUS_BEFORE_SUBCONF_DESTROY",
            "MEETING_STATUS_SUBCONF_CREATED",
            "MEETING_STATUS_SUBCONF_DESTROYED",
        ],
        code,
    )
}
pub const MEETING_STATUS_INMEETING: i32 = 3;

/// `ZMToolSuiteUserRole` (raw values from ToolSuiteProxyDef.h).
pub fn user_role_name(code: i32) -> String { pick(&["ZMToolSuiteUserRole_None", "ZMToolSuiteUserRole_Host", "ZMToolSuiteUserRole_CoHost", "ZMToolSuiteUserRole_Attendee", "ZMToolSuiteUserRole_Panelist", "ZMToolSuiteUserRole_BreakoutRoomModerator"], code) }

/// `ZMToolSuiteUserAction`
pub fn user_action_name(code: i32) -> String {
    pick(
        &["USER_ACTION_NO_ACTION", "USER_ACTION_SYNC_ALL", "USER_ACTION_ADD_USER", "USER_ACTION_LEFT_USER", "USER_ACTION_UPDATE_USER"],
        code,
    )
}

/// `ZMToolSuiteProxyErrors` (note: 1 = success, 0 = unknown).
pub fn proxy_error_name(code: i32) -> String {
    pick(
        &[
            "kZMToolSuiteProxyErrorsUnknown",
            "kZMToolSuiteProxyErrorsSuccess",
            "kZMToolSuiteProxyErrorsNotImplemented",
            "kZMToolSuiteProxyErrorsWrongUsage",
            "kZMToolSuiteProxyErrorsInvalidArgument",
            "kZMToolSuiteProxyErrorsInternalError",
            "kZMToolSuiteProxyErrorsNoPermission",
            "kZMToolSuiteProxyErrorsRateLimitReached",
            "kZMToolSuiteProxyErrorsOperationTimeout",
            "kZMToolSuiteProxyErrorsVersionIncompatible",
        ],
        code,
    )
}

/// `ZMToolSuiteMeetingType`
pub fn meeting_type_name(code: i32) -> String {
    pick(
        &["MEETING_TYPE_NONE", "MEETING_TYPE_NORMAL", "MEETING_TYPE_WEBINAR", "MEETING_TYPE_WEBINARBO", "MEETING_TYPE_BO", "MEETING_TYPE_GREENROOM", "MEETING_TYPE_NEWBO"],
        code,
    )
}

/// `ZMToolSuiteMeetingInstance` values for toolkit factories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum MeetingInstance {
    Default = 1,
    GreenRoom = 2,
    NewBreakout = 3,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names() {
        assert_eq!(auth_result_name(0), "AUTH_RESULT_SUCCESS");
        assert_eq!(ipc_status_name(IPC_STATUS_CONNECTED), "IPC_STATUS_CONNECTED");
        assert_eq!(meeting_status_name(MEETING_STATUS_INMEETING), "MEETING_STATUS_INMEETING");
        assert_eq!(proxy_error_name(1), "kZMToolSuiteProxyErrorsSuccess");
        assert_eq!(user_role_name(0), "ZMToolSuiteUserRole_None");
        assert_eq!(user_role_name(5), "ZMToolSuiteUserRole_BreakoutRoomModerator");
        assert_eq!(user_role_name(99), "UNKNOWN(99)");
        assert_eq!(auth_result_name(77), "UNKNOWN(77)");
        assert_eq!(auth_result_name(-1), "UNKNOWN(-1)");
    }
}
