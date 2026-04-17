use serde::{Deserialize, Serialize};

/// Full price breakdown for an order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceBreakdown {
    pub subtotal: f64,
    pub tax_rate: f64,
    pub tax_amount: f64,
    pub total: f64,
}

/// Calculate the price for a single item: base price + sum of option price deltas.
pub fn calculate_item_price(base_price: f64, option_deltas: &[f64]) -> f64 {
    base_price + option_deltas.iter().sum::<f64>()
}

/// Calculate the subtotal: sum of (unit_price * quantity) for each item.
pub fn calculate_subtotal(items: &[(f64, i32)]) -> f64 {
    items.iter().map(|(price, qty)| price * (*qty as f64)).sum()
}

/// Calculate tax on a subtotal.
pub fn calculate_tax(subtotal: f64, tax_rate: f64) -> f64 {
    (subtotal * tax_rate * 100.0).round() / 100.0
}

/// Calculate the final total.
pub fn calculate_total(subtotal: f64, tax: f64) -> f64 {
    ((subtotal + tax) * 100.0).round() / 100.0
}

/// Compute a full price breakdown from line items and a tax rate.
pub fn compute_breakdown(items: &[(f64, i32)], tax_rate: f64) -> PriceBreakdown {
    let subtotal = calculate_subtotal(items);
    let tax_amount = calculate_tax(subtotal, tax_rate);
    let total = calculate_total(subtotal, tax_amount);

    PriceBreakdown {
        subtotal,
        tax_rate,
        tax_amount,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_subtotal_empty() {
        assert_eq!(calculate_subtotal(&[]), 0.0);
    }

    #[test]
    fn test_calculate_subtotal_single_item() {
        assert_eq!(calculate_subtotal(&[(5.0, 3)]), 15.0);
    }

    #[test]
    fn test_calculate_subtotal_multiple_items() {
        let items = [(10.0, 2), (3.50, 4)];
        assert_eq!(calculate_subtotal(&items), 34.0);
    }

    #[test]
    fn test_calculate_tax_rounds_to_cents() {
        // 10.00 * 0.075 = 0.75
        assert_eq!(calculate_tax(10.0, 0.075), 0.75);
        // 3.33 * 0.1 = 0.333 → rounds to 0.33
        assert_eq!(calculate_tax(3.33, 0.1), 0.33);
    }

    #[test]
    fn test_calculate_item_price_no_options() {
        assert_eq!(calculate_item_price(9.99, &[]), 9.99);
    }

    #[test]
    fn test_calculate_item_price_with_options() {
        assert_eq!(calculate_item_price(9.99, &[0.50, 1.00]), 11.49);
    }

    #[test]
    fn test_compute_breakdown_consistency() {
        let items = [(10.0, 2), (5.0, 1)];
        let bd = compute_breakdown(&items, 0.08);
        assert_eq!(bd.subtotal, 25.0);
        assert_eq!(bd.tax_rate, 0.08);
        assert_eq!(bd.tax_amount, 2.0);
        assert_eq!(bd.total, 27.0);
    }

    #[test]
    fn test_compute_breakdown_total_equals_subtotal_plus_tax() {
        let items = [(7.77, 3)];
        let bd = compute_breakdown(&items, 0.06);
        assert!((bd.total - (bd.subtotal + bd.tax_amount)).abs() < 0.01);
    }

    // ── additional coverage ───────────────────────────────────────────────

    #[test]
    fn test_calculate_item_price_with_negative_discount_option() {
        // A "no sweetness" option might carry a -0.25 discount.
        assert_eq!(calculate_item_price(5.00, &[-0.25]), 4.75);
    }

    #[test]
    fn test_calculate_subtotal_zero_quantity_yields_zero_line() {
        let items = [(9.99, 0), (5.0, 2)];
        assert_eq!(calculate_subtotal(&items), 10.0);
    }

    #[test]
    fn test_calculate_tax_zero_rate_is_zero() {
        assert_eq!(calculate_tax(100.0, 0.0), 0.0);
    }

    #[test]
    fn test_calculate_tax_rounds_half_up_pattern() {
        // 1.25 * 0.0625 = 0.078125 → rounds to 0.08
        assert_eq!(calculate_tax(1.25, 0.0625), 0.08);
    }

    #[test]
    fn test_compute_breakdown_empty_cart() {
        let bd = compute_breakdown(&[], 0.08);
        assert_eq!(bd.subtotal, 0.0);
        assert_eq!(bd.tax_amount, 0.0);
        assert_eq!(bd.total, 0.0);
    }

    #[test]
    fn test_compute_breakdown_high_value_stays_consistent() {
        let items = [(99.99, 10)];
        let bd = compute_breakdown(&items, 0.10);
        assert!((bd.subtotal - 999.9).abs() < 1e-9);
        assert!((bd.tax_amount - 99.99).abs() < 0.01);
        assert!((bd.total - (bd.subtotal + bd.tax_amount)).abs() < 0.02);
    }

    #[test]
    fn test_price_breakdown_serializes_round_trip() {
        let bd = compute_breakdown(&[(4.50, 2)], 0.10);
        let json = serde_json::to_string(&bd).unwrap();
        let parsed: PriceBreakdown = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.subtotal, bd.subtotal);
        assert_eq!(parsed.total, bd.total);
    }
}
