use std::collections::HashMap;

/// Holds all translations: locale -> key -> translated string.
#[derive(Debug, Clone)]
pub struct Translations {
    pub map: HashMap<String, HashMap<String, String>>,
}

impl Translations {
    /// Look up a translation. Returns the key itself when no match is found.
    pub fn t(&self, locale: &str, key: &str) -> String {
        self.map
            .get(locale)
            .and_then(|m| m.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }
}

/// Free function shortcut (requires a reference to `Translations`).
pub fn t(translations: &Translations, locale: &str, key: &str) -> String {
    translations.t(locale, key)
}

/// Build the default translations for English and Chinese.
pub fn init_translations() -> Translations {
    let mut en: HashMap<String, String> = HashMap::new();
    let mut zh: HashMap<String, String> = HashMap::new();

    // ----- Navigation -----
    en.insert("nav.home".into(), "Home".into());
    zh.insert("nav.home".into(), "\u{9996}\u{9875}".into());

    en.insert("nav.menu".into(), "Menu".into());
    zh.insert("nav.menu".into(), "\u{83dc}\u{5355}".into());

    en.insert("nav.cart".into(), "Cart".into());
    zh.insert("nav.cart".into(), "\u{8d2d}\u{7269}\u{8f66}".into());

    en.insert("nav.orders".into(), "Orders".into());
    zh.insert("nav.orders".into(), "\u{8ba2}\u{5355}".into());

    en.insert("nav.training".into(), "Training".into());
    zh.insert("nav.training".into(), "\u{57f9}\u{8bad}".into());

    en.insert("nav.admin".into(), "Admin".into());
    zh.insert("nav.admin".into(), "\u{7ba1}\u{7406}".into());

    en.insert("nav.staff".into(), "Staff".into());
    zh.insert("nav.staff".into(), "\u{5458}\u{5de5}".into());

    // ----- Buttons -----
    en.insert("btn.add_to_cart".into(), "Add to Cart".into());
    zh.insert("btn.add_to_cart".into(), "\u{52a0}\u{5165}\u{8d2d}\u{7269}\u{8f66}".into());

    en.insert("btn.checkout".into(), "Checkout".into());
    zh.insert("btn.checkout".into(), "\u{7ed3}\u{8d26}".into());

    en.insert("btn.confirm".into(), "Confirm".into());
    zh.insert("btn.confirm".into(), "\u{786e}\u{8ba4}".into());

    en.insert("btn.cancel".into(), "Cancel".into());
    zh.insert("btn.cancel".into(), "\u{53d6}\u{6d88}".into());

    en.insert("btn.submit".into(), "Submit".into());
    zh.insert("btn.submit".into(), "\u{63d0}\u{4ea4}".into());

    en.insert("btn.scan".into(), "Scan".into());
    zh.insert("btn.scan".into(), "\u{626b}\u{7801}".into());

    en.insert("btn.start_exam".into(), "Start Exam".into());
    zh.insert("btn.start_exam".into(), "\u{5f00}\u{59cb}\u{8003}\u{8bd5}".into());

    en.insert("btn.finish_exam".into(), "Finish Exam".into());
    zh.insert("btn.finish_exam".into(), "\u{7ed3}\u{675f}\u{8003}\u{8bd5}".into());

    en.insert("btn.review".into(), "Review".into());
    zh.insert("btn.review".into(), "\u{590d}\u{4e60}".into());

    en.insert("btn.import".into(), "Import".into());
    zh.insert("btn.import".into(), "\u{5bfc}\u{5165}".into());

    en.insert("btn.generate".into(), "Generate".into());
    zh.insert("btn.generate".into(), "\u{751f}\u{6210}".into());

    // ----- Labels -----
    en.insert("label.size".into(), "Size".into());
    zh.insert("label.size".into(), "\u{5c3a}\u{5bf8}".into());

    en.insert("label.milk_type".into(), "Milk Type".into());
    zh.insert("label.milk_type".into(), "\u{5976}\u{7c7b}\u{578b}".into());

    en.insert("label.sweetness".into(), "Sweetness".into());
    zh.insert("label.sweetness".into(), "\u{751c}\u{5ea6}".into());

    en.insert("label.quantity".into(), "Quantity".into());
    zh.insert("label.quantity".into(), "\u{6570}\u{91cf}".into());

    en.insert("label.subtotal".into(), "Subtotal".into());
    zh.insert("label.subtotal".into(), "\u{5c0f}\u{8ba1}".into());

    en.insert("label.tax".into(), "Tax".into());
    zh.insert("label.tax".into(), "\u{7a0e}\u{8d39}".into());

    en.insert("label.total".into(), "Total".into());
    zh.insert("label.total".into(), "\u{603b}\u{8ba1}".into());

    en.insert("label.pickup_time".into(), "Pickup Time".into());
    zh.insert("label.pickup_time".into(), "\u{53d6}\u{9910}\u{65f6}\u{95f4}".into());

    en.insert("label.voucher_code".into(), "Voucher Code".into());
    zh.insert("label.voucher_code".into(), "\u{53d6}\u{9910}\u{7801}".into());

    en.insert("label.hold_timer".into(), "Hold Timer".into());
    zh.insert("label.hold_timer".into(), "\u{4fdd}\u{7559}\u{8ba1}\u{65f6}".into());

    en.insert("label.order_status".into(), "Order Status".into());
    zh.insert("label.order_status".into(), "\u{8ba2}\u{5355}\u{72b6}\u{6001}".into());

    en.insert("label.difficulty".into(), "Difficulty".into());
    zh.insert("label.difficulty".into(), "\u{96be}\u{5ea6}".into());

    en.insert("label.subject".into(), "Subject".into());
    zh.insert("label.subject".into(), "\u{79d1}\u{76ee}".into());

    en.insert("label.chapter".into(), "Chapter".into());
    zh.insert("label.chapter".into(), "\u{7ae0}\u{8282}".into());

    en.insert("label.score".into(), "Score".into());
    zh.insert("label.score".into(), "\u{5206}\u{6570}".into());

    en.insert("label.time_limit".into(), "Time Limit".into());
    zh.insert("label.time_limit".into(), "\u{65f6}\u{95f4}\u{9650}\u{5236}".into());

    // ----- Statuses -----
    en.insert("status.pending".into(), "Pending".into());
    zh.insert("status.pending".into(), "\u{5f85}\u{5904}\u{7406}".into());

    en.insert("status.accepted".into(), "Accepted".into());
    zh.insert("status.accepted".into(), "\u{5df2}\u{63a5}\u{53d7}".into());

    en.insert("status.in_prep".into(), "In Preparation".into());
    zh.insert("status.in_prep".into(), "\u{5236}\u{4f5c}\u{4e2d}".into());

    en.insert("status.ready".into(), "Ready".into());
    zh.insert("status.ready".into(), "\u{5df2}\u{5b8c}\u{6210}".into());

    en.insert("status.picked_up".into(), "Picked Up".into());
    zh.insert("status.picked_up".into(), "\u{5df2}\u{53d6}\u{9910}".into());

    en.insert("status.canceled".into(), "Canceled".into());
    zh.insert("status.canceled".into(), "\u{5df2}\u{53d6}\u{6d88}".into());

    en.insert("status.held".into(), "Held".into());
    zh.insert("status.held".into(), "\u{4fdd}\u{7559}\u{4e2d}".into());

    en.insert("status.confirmed".into(), "Confirmed".into());
    zh.insert("status.confirmed".into(), "\u{5df2}\u{786e}\u{8ba4}".into());

    en.insert("status.expired".into(), "Expired".into());
    zh.insert("status.expired".into(), "\u{5df2}\u{8fc7}\u{671f}".into());

    // ----- Page titles -----
    en.insert("page.menu".into(), "Menu".into());
    zh.insert("page.menu".into(), "\u{83dc}\u{5355}".into());

    en.insert("page.cart".into(), "Shopping Cart".into());
    zh.insert("page.cart".into(), "\u{8d2d}\u{7269}\u{8f66}".into());

    en.insert("page.checkout".into(), "Checkout".into());
    zh.insert("page.checkout".into(), "\u{7ed3}\u{8d26}".into());

    en.insert("page.orders".into(), "My Orders".into());
    zh.insert("page.orders".into(), "\u{6211}\u{7684}\u{8ba2}\u{5355}".into());

    en.insert("page.order_detail".into(), "Order Detail".into());
    zh.insert("page.order_detail".into(), "\u{8ba2}\u{5355}\u{8be6}\u{60c5}".into());

    en.insert("page.staff_dashboard".into(), "Staff Dashboard".into());
    zh.insert("page.staff_dashboard".into(), "\u{5458}\u{5de5}\u{9762}\u{677f}".into());

    en.insert("page.question_bank".into(), "Question Bank".into());
    zh.insert("page.question_bank".into(), "\u{9898}\u{5e93}".into());

    en.insert("page.mock_exams".into(), "Mock Exams".into());
    zh.insert("page.mock_exams".into(), "\u{6a21}\u{62df}\u{8003}\u{8bd5}".into());

    en.insert("page.analytics".into(), "Analytics".into());
    zh.insert("page.analytics".into(), "\u{6570}\u{636e}\u{5206}\u{6790}".into());

    en.insert("page.wrong_notebook".into(), "Wrong Notebook".into());
    zh.insert("page.wrong_notebook".into(), "\u{9519}\u{9898}\u{672c}".into());

    en.insert("page.favorites".into(), "Favorites".into());
    zh.insert("page.favorites".into(), "\u{6536}\u{85cf}".into());

    // ----- Errors -----
    en.insert("error.not_found".into(), "Not Found".into());
    zh.insert("error.not_found".into(), "\u{672a}\u{627e}\u{5230}".into());

    en.insert("error.unauthorized".into(), "Unauthorized".into());
    zh.insert("error.unauthorized".into(), "\u{672a}\u{6388}\u{6743}".into());

    en.insert("error.invalid_input".into(), "Invalid Input".into());
    zh.insert("error.invalid_input".into(), "\u{8f93}\u{5165}\u{65e0}\u{6548}".into());

    en.insert("error.slot_unavailable".into(), "Slot Unavailable".into());
    zh.insert("error.slot_unavailable".into(), "\u{65f6}\u{6bb5}\u{4e0d}\u{53ef}\u{7528}".into());

    // ----- Messages -----
    en.insert("msg.hold_warning".into(), "Your reservation hold is about to expire!".into());
    zh.insert("msg.hold_warning".into(), "\u{60a8}\u{7684}\u{9884}\u{7ea6}\u{4fdd}\u{7559}\u{5373}\u{5c06}\u{8fc7}\u{671f}\u{ff01}".into());

    en.insert("msg.item_released".into(), "Item has been released.".into());
    zh.insert("msg.item_released".into(), "\u{5546}\u{54c1}\u{5df2}\u{91ca}\u{653e}\u{3002}".into());

    en.insert("msg.mismatch_warning".into(), "Voucher mismatch detected!".into());
    zh.insert("msg.mismatch_warning".into(), "\u{68c0}\u{6d4b}\u{5230}\u{53d6}\u{9910}\u{7801}\u{4e0d}\u{5339}\u{914d}\u{ff01}".into());

    en.insert("msg.exam_complete".into(), "Exam completed successfully.".into());
    zh.insert("msg.exam_complete".into(), "\u{8003}\u{8bd5}\u{5df2}\u{5b8c}\u{6210}\u{3002}".into());

    let mut map = HashMap::new();
    map.insert("en".to_string(), en);
    map.insert("zh".to_string(), zh);

    Translations { map }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_translation_exists() {
        let t = init_translations();
        assert_eq!(t.t("en", "nav.home"), "Home");
        assert_eq!(t.t("en", "btn.checkout"), "Checkout");
        assert_eq!(t.t("en", "label.total"), "Total");
    }

    #[test]
    fn chinese_translation_exists() {
        let t = init_translations();
        // Chinese translations should not be empty or fall back to key
        let home = t.t("zh", "nav.home");
        assert_ne!(home, "nav.home", "zh nav.home should not fall back to key");
        assert!(!home.is_empty());
    }

    #[test]
    fn missing_key_returns_key_itself() {
        let t = init_translations();
        assert_eq!(t.t("en", "nonexistent.key"), "nonexistent.key");
        assert_eq!(t.t("zh", "nonexistent.key"), "nonexistent.key");
    }

    #[test]
    fn unknown_locale_falls_back_to_key() {
        let t = init_translations();
        assert_eq!(t.t("fr", "nav.home"), "nav.home");
    }

    #[test]
    fn all_english_keys_have_chinese_counterpart() {
        let t = init_translations();
        let en_keys = t.map.get("en").unwrap();
        let zh_keys = t.map.get("zh").unwrap();
        for key in en_keys.keys() {
            assert!(
                zh_keys.contains_key(key),
                "missing zh translation for key: {}",
                key
            );
        }
    }

    #[test]
    fn all_chinese_keys_have_english_counterpart() {
        let t = init_translations();
        let en_keys = t.map.get("en").unwrap();
        let zh_keys = t.map.get("zh").unwrap();
        for key in zh_keys.keys() {
            assert!(
                en_keys.contains_key(key),
                "missing en translation for key: {}",
                key
            );
        }
    }

    #[test]
    fn free_function_t_works() {
        let translations = init_translations();
        assert_eq!(t(&translations, "en", "nav.menu"), "Menu");
    }
}
