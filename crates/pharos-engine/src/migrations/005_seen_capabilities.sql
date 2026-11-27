-- What the last applied sync declared (CapabilitySet JSON), approved or not. A change the
-- user declined is "seen" and is not asked about again until it changes.
ALTER TABLE sources ADD COLUMN seen_capabilities TEXT;
UPDATE sources SET seen_capabilities = approved_capabilities;
