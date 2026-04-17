/// Validate whether transitioning from `current` to `next` status is allowed.
///
/// Valid transitions:
/// - pending -> accepted
/// - accepted -> in_prep
/// - in_prep -> ready
/// - ready -> picked_up
/// - any (except picked_up, canceled) -> canceled
///
/// The `roles` parameter is checked for the `ready -> canceled` transition:
/// only users with the "Admin" role (case-insensitive) may cancel an order
/// that has already reached "ready" status.
pub fn validate_transition(current: &str, next: &str, roles: &[String]) -> bool {
    match (current, next) {
        ("Pending", "Accepted") => true,
        ("Accepted", "InPrep") => true,
        ("InPrep", "Ready") => true,
        ("Ready", "PickedUp") => true,
        // Any non-terminal status can transition to Canceled
        ("Pending", "Canceled") => true,
        ("Accepted", "Canceled") => true,
        ("InPrep", "Canceled") => true,
        // Ready -> Canceled requires Admin role
        ("Ready", "Canceled") => {
            roles.iter().any(|r| r.eq_ignore_ascii_case("admin"))
        }
        _ => false,
    }
}

/// Returns a human-readable error message explaining why cancellation was
/// blocked for the given `current` status and `roles`.
pub fn cancel_error_message(current: &str, roles: &[String]) -> String {
    if current == "Ready" && !roles.iter().any(|r| r.eq_ignore_ascii_case("admin")) {
        "Cannot cancel an order that is already in 'Ready' status. \
         Only users with the Admin role may cancel ready orders."
            .to_string()
    } else if current == "PickedUp" || current == "Canceled" {
        format!(
            "Cannot cancel an order with terminal status '{}'.",
            current
        )
    } else {
        format!(
            "Invalid cancellation from status '{}'. This transition is not allowed.",
            current
        )
    }
}

/// Check whether the voucher's order matches the presented order.
///
/// Returns `(matches, mismatch_reason)`.
pub fn check_voucher_match(
    voucher_order_id: i64,
    presented_order_id: i64,
) -> (bool, Option<String>) {
    if voucher_order_id == presented_order_id {
        (true, None)
    } else {
        (
            false,
            Some(format!(
                "Voucher belongs to order {} but was presented for order {}",
                voucher_order_id, presented_order_id
            )),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    // ── validate_transition ────────────────────────────────────────────────

    #[test]
    fn valid_forward_transitions() {
        assert!(validate_transition("Pending", "Accepted", &roles(&[])));
        assert!(validate_transition("Accepted", "InPrep", &roles(&[])));
        assert!(validate_transition("InPrep", "Ready", &roles(&[])));
        assert!(validate_transition("Ready", "PickedUp", &roles(&[])));
    }

    #[test]
    fn cancel_from_non_terminal_statuses() {
        assert!(validate_transition("Pending", "Canceled", &roles(&[])));
        assert!(validate_transition("Accepted", "Canceled", &roles(&[])));
        assert!(validate_transition("InPrep", "Canceled", &roles(&[])));
    }

    #[test]
    fn cancel_ready_requires_admin() {
        assert!(!validate_transition("Ready", "Canceled", &roles(&["Staff"])));
        assert!(validate_transition("Ready", "Canceled", &roles(&["Admin"])));
        // Case-insensitive admin check
        assert!(validate_transition("Ready", "Canceled", &roles(&["admin"])));
    }

    #[test]
    fn terminal_statuses_cannot_transition() {
        assert!(!validate_transition("PickedUp", "Canceled", &roles(&["Admin"])));
        assert!(!validate_transition("Canceled", "Pending", &roles(&[])));
    }

    #[test]
    fn invalid_skip_transitions() {
        assert!(!validate_transition("Pending", "InPrep", &roles(&[])));
        assert!(!validate_transition("Pending", "Ready", &roles(&[])));
    }

    // ── cancel_error_message ───────────────────────────────────────────────

    #[test]
    fn cancel_error_ready_no_admin() {
        let msg = cancel_error_message("Ready", &roles(&["Staff"]));
        assert!(msg.contains("Admin"));
    }

    #[test]
    fn cancel_error_terminal_picked_up() {
        let msg = cancel_error_message("PickedUp", &roles(&[]));
        assert!(msg.contains("PickedUp"));
    }

    #[test]
    fn cancel_error_terminal_canceled() {
        let msg = cancel_error_message("Canceled", &roles(&[]));
        assert!(msg.contains("Canceled"));
    }

    // ── check_voucher_match ────────────────────────────────────────────────

    #[test]
    fn voucher_match_same_order() {
        let (matches, reason) = check_voucher_match(42, 42);
        assert!(matches);
        assert!(reason.is_none());
    }

    #[test]
    fn voucher_mismatch_different_order() {
        let (matches, reason) = check_voucher_match(1, 2);
        assert!(!matches);
        let msg = reason.unwrap();
        assert!(msg.contains('1') && msg.contains('2'));
    }

    // ── full state-machine matrix ──────────────────────────────────────────

    #[test]
    fn all_backward_transitions_rejected() {
        // No status can rewind to an earlier one.
        let pairs = &[
            ("Accepted", "Pending"),
            ("InPrep", "Accepted"),
            ("Ready", "InPrep"),
            ("PickedUp", "Ready"),
        ];
        for (from, to) in pairs {
            assert!(
                !validate_transition(from, to, &roles(&["Admin"])),
                "{}→{} must be forbidden",
                from,
                to
            );
        }
    }

    #[test]
    fn unknown_status_is_rejected() {
        assert!(!validate_transition("Floating", "Pending", &roles(&[])));
        assert!(!validate_transition("Pending", "UnknownStatus", &roles(&[])));
    }

    #[test]
    fn admin_plus_other_role_can_cancel_ready() {
        // Admin among other roles still permits Ready→Canceled.
        assert!(validate_transition(
            "Ready",
            "Canceled",
            &roles(&["Customer", "Admin"])
        ));
    }

    #[test]
    fn staff_alone_cannot_cancel_ready() {
        assert!(!validate_transition("Ready", "Canceled", &roles(&["Staff"])));
    }

    #[test]
    fn cancel_error_mentions_admin_requirement_only_when_ready() {
        // Pending→Canceled is allowed, so the message code-path is for
        // "invalid" only; ensure we still produce a useful string.
        let msg = cancel_error_message("Pending", &roles(&["Staff"]));
        assert!(msg.contains("Pending"));
    }

    #[test]
    fn check_voucher_match_zero_ids_match() {
        // Edge case: 0 == 0 still matches. Not a realistic scenario but the
        // function is agnostic to value.
        let (ok, reason) = check_voucher_match(0, 0);
        assert!(ok);
        assert!(reason.is_none());
    }
}
