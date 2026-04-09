-- ============================================================
-- Migration 006: Seed Data
-- ============================================================

-- -----------------------------------------------------------
-- 1. Default Roles
-- -----------------------------------------------------------
INSERT INTO `roles` (`name`, `description`) VALUES
    ('Admin',           'Full system administrator with unrestricted access'),
    ('Staff',           'Store staff who can manage orders and fulfillment'),
    ('Customer',        'Regular customer who can browse, order, and take exams'),
    ('AcademicAffairs', 'Academic affairs personnel who manage exam content'),
    ('Teacher',         'Teacher who can create and manage exam questions')
ON DUPLICATE KEY UPDATE `description` = VALUES(`description`);

-- -----------------------------------------------------------
-- 2. Store Hours (Mon-Fri 7AM-9PM, Sat-Sun 8AM-8PM)
-- -----------------------------------------------------------
INSERT INTO `store_hours` (`day_of_week`, `open_time`, `close_time`, `is_closed`) VALUES
    (0, '08:00:00', '20:00:00', FALSE),  -- Sunday
    (1, '07:00:00', '21:00:00', FALSE),  -- Monday
    (2, '07:00:00', '21:00:00', FALSE),  -- Tuesday
    (3, '07:00:00', '21:00:00', FALSE),  -- Wednesday
    (4, '07:00:00', '21:00:00', FALSE),  -- Thursday
    (5, '07:00:00', '21:00:00', FALSE),  -- Friday
    (6, '08:00:00', '20:00:00', FALSE);  -- Saturday

-- -----------------------------------------------------------
-- 3. Sales Tax Config (8.75%)
-- -----------------------------------------------------------
INSERT INTO `sales_tax_config` (`tax_name`, `rate`, `is_active`) VALUES
    ('General Sales Tax', 0.0875, TRUE);

-- -----------------------------------------------------------
-- 4. Sample SPU: Classic Latte
-- -----------------------------------------------------------
INSERT INTO `spu` (`id`, `name_en`, `name_zh`, `description_en`, `description_zh`, `category`, `base_price`, `prep_time_minutes`, `is_active`) VALUES
    (1, 'Classic Latte', '经典拿铁', 'A smooth and creamy espresso-based drink with steamed milk.', '一杯顺滑浓郁的意式浓缩咖啡与蒸牛奶的完美结合。', 'Coffee', 4.50, 5, TRUE);

-- Option Group: Size (for Classic Latte)
INSERT INTO `option_groups` (`id`, `spu_id`, `name_en`, `name_zh`, `is_required`, `sort_order`) VALUES
    (1, 1, 'Size', '杯型', TRUE, 1);

INSERT INTO `option_values` (`id`, `group_id`, `label_en`, `label_zh`, `price_delta`, `is_default`, `sort_order`) VALUES
    (1, 1, 'Small',  '小杯', 0.00, FALSE, 1),
    (2, 1, 'Medium', '中杯', 1.00, TRUE,  2),
    (3, 1, 'Large',  '大杯', 2.00, FALSE, 3);

-- Option Group: Milk Type (for Classic Latte)
INSERT INTO `option_groups` (`id`, `spu_id`, `name_en`, `name_zh`, `is_required`, `sort_order`) VALUES
    (2, 1, 'Milk Type', '奶类', TRUE, 2);

INSERT INTO `option_values` (`id`, `group_id`, `label_en`, `label_zh`, `price_delta`, `is_default`, `sort_order`) VALUES
    (4, 2, 'Whole Milk',  '全脂牛奶', 0.00, TRUE,  1),
    (5, 2, 'Oat Milk',    '燕麦奶',   0.75, FALSE, 2),
    (6, 2, 'Almond Milk', '杏仁奶',   0.75, FALSE, 3),
    (7, 2, 'Soy Milk',    '豆奶',     0.50, FALSE, 4);

-- Option Group: Sweetness (for Classic Latte)
INSERT INTO `option_groups` (`id`, `spu_id`, `name_en`, `name_zh`, `is_required`, `sort_order`) VALUES
    (3, 1, 'Sweetness', '甜度', TRUE, 3);

INSERT INTO `option_values` (`id`, `group_id`, `label_en`, `label_zh`, `price_delta`, `is_default`, `sort_order`) VALUES
    (8,  3, 'None',    '无糖',   0.00, FALSE, 1),
    (9,  3, 'Light',   '少糖',   0.00, FALSE, 2),
    (10, 3, 'Regular', '正常糖', 0.00, TRUE,  3),
    (11, 3, 'Extra',   '多糖',   0.25, FALSE, 4);

-- -----------------------------------------------------------
-- 5. Sample SPU: Iced Americano
-- -----------------------------------------------------------
INSERT INTO `spu` (`id`, `name_en`, `name_zh`, `description_en`, `description_zh`, `category`, `base_price`, `prep_time_minutes`, `is_active`) VALUES
    (2, 'Iced Americano', '冰美式', 'Bold espresso shots over ice with cold water.', '浓缩咖啡加冰水，清爽提神。', 'Coffee', 3.50, 3, TRUE);

-- Option Group: Size (for Iced Americano)
INSERT INTO `option_groups` (`id`, `spu_id`, `name_en`, `name_zh`, `is_required`, `sort_order`) VALUES
    (4, 2, 'Size', '杯型', TRUE, 1);

INSERT INTO `option_values` (`id`, `group_id`, `label_en`, `label_zh`, `price_delta`, `is_default`, `sort_order`) VALUES
    (12, 4, 'Small',  '小杯', 0.00, FALSE, 1),
    (13, 4, 'Medium', '中杯', 1.00, TRUE,  2),
    (14, 4, 'Large',  '大杯', 2.00, FALSE, 3);

-- Option Group: Milk Type (for Iced Americano)
INSERT INTO `option_groups` (`id`, `spu_id`, `name_en`, `name_zh`, `is_required`, `sort_order`) VALUES
    (5, 2, 'Milk Type', '奶类', TRUE, 2);

INSERT INTO `option_values` (`id`, `group_id`, `label_en`, `label_zh`, `price_delta`, `is_default`, `sort_order`) VALUES
    (15, 5, 'Whole Milk',  '全脂牛奶', 0.00, TRUE,  1),
    (16, 5, 'Oat Milk',    '燕麦奶',   0.75, FALSE, 2),
    (17, 5, 'Almond Milk', '杏仁奶',   0.75, FALSE, 3),
    (18, 5, 'Soy Milk',    '豆奶',     0.50, FALSE, 4);

-- Option Group: Sweetness (for Iced Americano)
INSERT INTO `option_groups` (`id`, `spu_id`, `name_en`, `name_zh`, `is_required`, `sort_order`) VALUES
    (6, 2, 'Sweetness', '甜度', TRUE, 3);

INSERT INTO `option_values` (`id`, `group_id`, `label_en`, `label_zh`, `price_delta`, `is_default`, `sort_order`) VALUES
    (19, 6, 'None',    '无糖',   0.00, FALSE, 1),
    (20, 6, 'Light',   '少糖',   0.00, FALSE, 2),
    (21, 6, 'Regular', '正常糖', 0.00, TRUE,  3),
    (22, 6, 'Extra',   '多糖',   0.25, FALSE, 4);

-- -----------------------------------------------------------
-- 6. Sample SPU: Matcha Latte
-- -----------------------------------------------------------
INSERT INTO `spu` (`id`, `name_en`, `name_zh`, `description_en`, `description_zh`, `category`, `base_price`, `prep_time_minutes`, `is_active`) VALUES
    (3, 'Matcha Latte', '抹茶拿铁', 'Premium matcha green tea blended with steamed milk.', '优质抹茶与蒸牛奶的完美融合。', 'Tea', 5.00, 7, TRUE);

-- Option Group: Size (for Matcha Latte)
INSERT INTO `option_groups` (`id`, `spu_id`, `name_en`, `name_zh`, `is_required`, `sort_order`) VALUES
    (7, 3, 'Size', '杯型', TRUE, 1);

INSERT INTO `option_values` (`id`, `group_id`, `label_en`, `label_zh`, `price_delta`, `is_default`, `sort_order`) VALUES
    (23, 7, 'Small',  '小杯', 0.00, FALSE, 1),
    (24, 7, 'Medium', '中杯', 1.00, TRUE,  2),
    (25, 7, 'Large',  '大杯', 2.00, FALSE, 3);

-- Option Group: Milk Type (for Matcha Latte)
INSERT INTO `option_groups` (`id`, `spu_id`, `name_en`, `name_zh`, `is_required`, `sort_order`) VALUES
    (8, 3, 'Milk Type', '奶类', TRUE, 2);

INSERT INTO `option_values` (`id`, `group_id`, `label_en`, `label_zh`, `price_delta`, `is_default`, `sort_order`) VALUES
    (26, 8, 'Whole Milk',  '全脂牛奶', 0.00, TRUE,  1),
    (27, 8, 'Oat Milk',    '燕麦奶',   0.75, FALSE, 2),
    (28, 8, 'Almond Milk', '杏仁奶',   0.75, FALSE, 3),
    (29, 8, 'Soy Milk',    '豆奶',     0.50, FALSE, 4);

-- Option Group: Sweetness (for Matcha Latte)
INSERT INTO `option_groups` (`id`, `spu_id`, `name_en`, `name_zh`, `is_required`, `sort_order`) VALUES
    (9, 3, 'Sweetness', '甜度', TRUE, 3);

INSERT INTO `option_values` (`id`, `group_id`, `label_en`, `label_zh`, `price_delta`, `is_default`, `sort_order`) VALUES
    (30, 9, 'None',    '无糖',   0.00, FALSE, 1),
    (31, 9, 'Light',   '少糖',   0.00, FALSE, 2),
    (32, 9, 'Regular', '正常糖', 0.00, TRUE,  3),
    (33, 9, 'Extra',   '多糖',   0.25, FALSE, 4);

-- -----------------------------------------------------------
-- 7. Sample Subjects
-- -----------------------------------------------------------
INSERT INTO `subjects` (`name_en`, `name_zh`, `description_en`, `description_zh`) VALUES
    ('Coffee Knowledge',   '咖啡知识',   'Learn about coffee origins, varieties, and flavor profiles.',   '了解咖啡产地、品种和风味特征。'),
    ('Brewing Techniques', '冲泡技术',   'Master various coffee brewing methods and techniques.',         '掌握各种咖啡冲泡方法和技巧。'),
    ('Customer Service',   '客户服务',   'Best practices for providing excellent customer experiences.',   '提供卓越客户体验的最佳实践。');

-- -----------------------------------------------------------
-- 8. Admin User (password: AdminPass123!)
-- -----------------------------------------------------------
INSERT INTO `users` (`id`, `username`, `password_hash`, `display_name`, `email`, `preferred_locale`) VALUES
    (1, 'admin', '$2b$12$sfKp/ZnimC2SQ8o74NZ49uZw8FpYMHIAJBGfF3ly5pI1rs6cDuqw2', 'Administrator', 'admin@brewflow.local', 'en');

-- Assign Admin role to admin user
INSERT INTO `user_roles` (`user_id`, `role_id`)
    SELECT 1, `id` FROM `roles` WHERE `name` = 'Admin';
