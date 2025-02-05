ALTER TABLE jobs 
    ALTER COLUMN batch_range_begin_epoch DROP NOT NULL,
    ALTER COLUMN batch_range_end_epoch DROP NOT NULL; 