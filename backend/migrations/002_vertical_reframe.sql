ALTER TABLE tasks ADD COLUMN auto_vertical_reframe INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tasks ADD COLUMN reframe_preset TEXT NOT NULL DEFAULT 'talking_head';
