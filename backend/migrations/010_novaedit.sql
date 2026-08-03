-- NovaEdit — agentic video editing support
ALTER TABLE tasks ADD COLUMN novaedit_payload TEXT;
ALTER TABLE tasks ADD COLUMN edit_plan TEXT;
ALTER TABLE tasks ADD COLUMN review_score TEXT;
