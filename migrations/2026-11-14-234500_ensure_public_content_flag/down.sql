-- Compatibility repair: the original streaming-policy migration also owns this
-- column in fresh databases, so rolling this repair back must not remove it.
SELECT 1;
