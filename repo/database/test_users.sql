-- CI test users and fixtures — applied on top of 006_seed_data.sql.
-- Passwords are bcrypt-hashed at cost 12.
--   admin    / AdminPass123!
--   customer / CustomerPass123!
--   staff    / StaffPass123!
--   teacher  / TeacherPass123!
--   academic / AcademicPass123!

-- -----------------------------------------------------------
-- 1. Users
-- -----------------------------------------------------------

-- Update admin user to use the CI test password
UPDATE `users`
SET `password_hash` = '$2b$12$VNOcg4vPubd83IJI89B0JecqaoV38MENRWVD3iiUXmeiSOfbJzQ9m'
WHERE `username` = 'admin';

-- Insert customer test user
INSERT INTO `users` (`id`, `username`, `password_hash`, `display_name`, `email`, `preferred_locale`)
VALUES (2, 'customer', '$2b$12$L9DeR18TSZZGtq9jeK5Breupf9mJN4O.WEut.MTmDxpTA/szIlc7.',
        'Test Customer', 'customer@brewflow.local', 'en')
ON DUPLICATE KEY UPDATE `password_hash` = VALUES(`password_hash`);

INSERT INTO `user_roles` (`user_id`, `role_id`)
    SELECT 2, `id` FROM `roles` WHERE `name` = 'Customer'
ON DUPLICATE KEY UPDATE `user_id` = `user_id`;

-- Insert staff test user
INSERT INTO `users` (`id`, `username`, `password_hash`, `display_name`, `email`, `preferred_locale`)
VALUES (3, 'staff', '$2b$12$HnHvvT3d9O5MTqJjWU9Eoe1IiNpNuOCzemkFOoZIZCxTBe21IHbEO',
        'Test Staff', 'staff@brewflow.local', 'en')
ON DUPLICATE KEY UPDATE `password_hash` = VALUES(`password_hash`);

INSERT INTO `user_roles` (`user_id`, `role_id`)
    SELECT 3, `id` FROM `roles` WHERE `name` = 'Staff'
ON DUPLICATE KEY UPDATE `user_id` = `user_id`;

-- Insert teacher test user
INSERT INTO `users` (`id`, `username`, `password_hash`, `display_name`, `email`, `preferred_locale`)
VALUES (4, 'teacher', '$2b$12$RAvN2scxrQVkb6O/dGyGC.XTcOc/Yrt0fdWmoNmg48r5Rm/dzN/jy',
        'Test Teacher', 'teacher@brewflow.local', 'en')
ON DUPLICATE KEY UPDATE `password_hash` = VALUES(`password_hash`);

INSERT INTO `user_roles` (`user_id`, `role_id`)
    SELECT 4, `id` FROM `roles` WHERE `name` = 'Teacher'
ON DUPLICATE KEY UPDATE `user_id` = `user_id`;

-- Insert academic affairs test user
INSERT INTO `users` (`id`, `username`, `password_hash`, `display_name`, `email`, `preferred_locale`)
VALUES (5, 'academic', '$2b$12$Y/E046aMU802uXceRxiQ2ecOhTMMXILbFZ5VmufyJnoao.KdwlorC',
        'Test Academic Affairs', 'academic@brewflow.local', 'en')
ON DUPLICATE KEY UPDATE `password_hash` = VALUES(`password_hash`);

INSERT INTO `user_roles` (`user_id`, `role_id`)
    SELECT 5, `id` FROM `roles` WHERE `name` = 'AcademicAffairs'
ON DUPLICATE KEY UPDATE `user_id` = `user_id`;

-- -----------------------------------------------------------
-- 2. Fixture: order with expired reservation (for hold-expiry test)
-- -----------------------------------------------------------

-- Reservation that expired in the past, still in 'Held' status
INSERT INTO `reservations` (`id`, `user_id`, `pickup_slot_start`, `pickup_slot_end`,
                            `voucher_code`, `hold_expires_at`, `status`)
VALUES (9000, 2, '2020-01-01 10:00:00', '2020-01-01 10:30:00',
        '966b267e525577815b03e1244b8d73d09372d7ea84a34a293acff21ae87be217',
        '2020-01-01 09:00:00', 'Held')
ON DUPLICATE KEY UPDATE `status` = 'Held', `hold_expires_at` = '2020-01-01 09:00:00';

-- Order tied to expired reservation, owned by customer (user_id=2)
INSERT INTO `orders` (`id`, `user_id`, `reservation_id`, `order_number`,
                      `subtotal`, `tax_amount`, `total`, `status`)
VALUES (9000, 2, 9000, 'TEST-EXPIRED-HOLD-001', 4.50, 0.39, 4.89, 'Pending')
ON DUPLICATE KEY UPDATE `status` = 'Pending', `reservation_id` = 9000;

-- -----------------------------------------------------------
-- 3. Fixture: cancelled order with voucher (for voucher-scan test)
-- -----------------------------------------------------------

-- Reservation for the cancelled order
INSERT INTO `reservations` (`id`, `user_id`, `pickup_slot_start`, `pickup_slot_end`,
                            `voucher_code`, `hold_expires_at`, `status`)
VALUES (9001, 2, '2020-01-01 11:00:00', '2020-01-01 11:30:00',
        '0f9bc54d217322cb321de5813ce28a7914d3b92ad9da958450073a66a7c4d615',
        '2020-01-01 12:00:00', 'Confirmed')
ON DUPLICATE KEY UPDATE `status` = 'Confirmed';

-- Cancelled order
INSERT INTO `orders` (`id`, `user_id`, `reservation_id`, `order_number`,
                      `subtotal`, `tax_amount`, `total`, `status`)
VALUES (9001, 2, 9001, 'TEST-CANCELLED-001', 4.50, 0.39, 4.89, 'Canceled')
ON DUPLICATE KEY UPDATE `status` = 'Canceled';

-- Voucher pointing at the cancelled order
-- Code stored as SHA256("TEST-CANCELLED-VOUCHER-001")
INSERT INTO `vouchers` (`id`, `reservation_id`, `order_id`, `code`)
VALUES (9001, 9001, 9001,
        '0f9bc54d217322cb321de5813ce28a7914d3b92ad9da958450073a66a7c4d615')
ON DUPLICATE KEY UPDATE `order_id` = 9001;
