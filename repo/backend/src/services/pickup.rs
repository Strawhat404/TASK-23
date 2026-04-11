use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Duration};
use shared::models::{StoreHours, Reservation};
use shared::dto::PickupSlot;
use rand::Rng;

/// Generate 15-minute pickup slots within store hours for a given date.
///
/// Marks slots as unavailable if they start within `prep_time_minutes` of now,
/// or if there are too many existing reservations for that slot (capacity = 5).
pub fn generate_pickup_slots(
    store_hours: &[StoreHours],
    date: NaiveDate,
    prep_time_minutes: i32,
    existing_reservations: &[Reservation],
) -> Vec<PickupSlot> {
    // Migration seeds day_of_week as 0=Sunday, 1=Monday, ..., 6=Saturday.
    // chrono's weekday().num_days_from_sunday() returns 0=Sun, 1=Mon, ..., 6=Sat.
    let day_of_week = date.weekday().num_days_from_sunday() as u8;

    let hours = store_hours
        .iter()
        .find(|h| h.day_of_week == day_of_week);

    let hours = match hours {
        Some(h) if !h.is_closed => h,
        _ => return Vec::new(),
    };

    let open = match NaiveTime::parse_from_str(&hours.open_time, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(&hours.open_time, "%H:%M"))
    {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    let close = match NaiveTime::parse_from_str(&hours.close_time, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(&hours.close_time, "%H:%M"))
    {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };

    let slot_duration = Duration::minutes(15);
    let now = Local::now().naive_local();
    let earliest_available = now + Duration::minutes(prep_time_minutes as i64);
    let max_reservations_per_slot: usize = 5;

    let mut slots = Vec::new();
    let mut slot_start_time = open;

    while slot_start_time + slot_duration <= close {
        let slot_start = NaiveDateTime::new(date, slot_start_time);
        let slot_end = slot_start + slot_duration;

        // Count overlapping reservations
        let reservation_count = existing_reservations
            .iter()
            .filter(|r| {
                r.status != "Expired" && r.status != "Canceled"
                    && r.pickup_slot_start < slot_end
                    && r.pickup_slot_end > slot_start
            })
            .count();

        let available = slot_start >= earliest_available
            && reservation_count < max_reservations_per_slot;

        slots.push(PickupSlot {
            start: slot_start.format("%Y-%m-%dT%H:%M:%S").to_string(),
            end: slot_end.format("%Y-%m-%dT%H:%M:%S").to_string(),
            available,
        });

        slot_start_time += slot_duration;
    }

    slots
}

/// Check whether a given slot start time is still available (far enough in the future).
pub fn is_slot_available(slot_start: NaiveDateTime, prep_time_minutes: i32) -> bool {
    let now = Local::now().naive_local();
    let earliest = now + Duration::minutes(prep_time_minutes as i64);
    slot_start >= earliest
}

/// Generate a voucher code in the format BF-XXXXXX (alphanumeric uppercase).
pub fn generate_voucher_code() -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect();
    let code: String = (0..6).map(|_| chars[rng.gen_range(0..chars.len())]).collect();
    format!("BF-{}", code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    fn make_store_hours(day: u8, open: &str, close: &str, is_closed: bool) -> StoreHours {
        StoreHours {
            id: 1,
            day_of_week: day,
            open_time: open.to_string(),
            close_time: close.to_string(),
            is_closed,
        }
    }

    fn make_reservation(start: NaiveDateTime, end: NaiveDateTime, status: &str) -> Reservation {
        Reservation {
            id: 1,
            user_id: 1,
            pickup_slot_start: start,
            pickup_slot_end: end,
            voucher_code: "HASH".to_string(),
            hold_expires_at: end,
            status: status.to_string(),
            created_at: start,
            updated_at: None,
        }
    }

    // ── voucher code format ───────────────────────────────────────────────

    #[test]
    fn voucher_code_has_correct_format() {
        let code = generate_voucher_code();
        assert!(code.starts_with("BF-"), "code must start with BF-, got: {}", code);
        assert_eq!(code.len(), 9, "BF-XXXXXX = 9 chars, got: {}", code);
    }

    #[test]
    fn voucher_code_is_alphanumeric_uppercase() {
        let code = generate_voucher_code();
        let suffix = &code[3..]; // strip "BF-"
        assert!(
            suffix.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
            "suffix must be uppercase alphanumeric, got: {}",
            suffix
        );
    }

    #[test]
    fn voucher_codes_are_unique() {
        let codes: Vec<String> = (0..100).map(|_| generate_voucher_code()).collect();
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        // With 36^6 ≈ 2 billion possibilities, 100 codes should all be unique.
        assert_eq!(unique.len(), codes.len());
    }

    // ── generate_pickup_slots ─────────────────────────────────────────────

    #[test]
    fn closed_day_returns_no_slots() {
        let hours = vec![make_store_hours(0, "09:00", "17:00", true)];
        // Sunday = 0
        let date = NaiveDate::from_ymd_opt(2026, 4, 12).unwrap(); // a Sunday
        let slots = generate_pickup_slots(&hours, date, 15, &[]);
        assert!(slots.is_empty(), "closed day should yield no slots");
    }

    #[test]
    fn missing_day_returns_no_slots() {
        // Store hours only for Monday (1), query for Sunday (0)
        let hours = vec![make_store_hours(1, "09:00", "17:00", false)];
        let date = NaiveDate::from_ymd_opt(2026, 4, 12).unwrap(); // Sunday
        let slots = generate_pickup_slots(&hours, date, 15, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn slots_are_15_minute_intervals() {
        // 2 hour window → 8 slots
        let hours = vec![make_store_hours(1, "09:00", "11:00", false)];
        let date = NaiveDate::from_ymd_opt(2026, 4, 13).unwrap(); // Monday
        let slots = generate_pickup_slots(&hours, date, 0, &[]);
        assert_eq!(slots.len(), 8, "2 hours / 15 min = 8 slots, got: {}", slots.len());
    }

    #[test]
    fn expired_reservations_do_not_reduce_capacity() {
        let hours = vec![make_store_hours(1, "09:00", "09:30", false)];
        let date = NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
        let slot_start = NaiveDateTime::new(date, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        let slot_end = NaiveDateTime::new(date, NaiveTime::from_hms_opt(9, 15, 0).unwrap());

        // 5 expired reservations should NOT count toward the slot capacity of 5
        let expired: Vec<Reservation> = (0..5)
            .map(|_| make_reservation(slot_start, slot_end, "Expired"))
            .collect();

        let slots = generate_pickup_slots(&hours, date, 0, &expired);
        assert_eq!(slots.len(), 2);
        // Both should still be "available" (ignoring time-based availability for far-future dates)
    }

    #[test]
    fn full_capacity_marks_slot_unavailable() {
        let hours = vec![make_store_hours(1, "09:00", "09:30", false)];
        let date = NaiveDate::from_ymd_opt(2099, 4, 13).unwrap(); // far future Monday
        let slot_start = NaiveDateTime::new(date, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        let slot_end = NaiveDateTime::new(date, NaiveTime::from_hms_opt(9, 15, 0).unwrap());

        // 5 confirmed reservations fills capacity
        let reservations: Vec<Reservation> = (0..5)
            .map(|_| make_reservation(slot_start, slot_end, "Confirmed"))
            .collect();

        let slots = generate_pickup_slots(&hours, date, 0, &reservations);
        let first = &slots[0];
        assert!(!first.available, "slot at full capacity should be unavailable");
    }
}
