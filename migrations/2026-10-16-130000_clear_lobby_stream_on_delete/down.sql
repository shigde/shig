ALTER TABLE lobbies DROP CONSTRAINT lobbies_stream_id_fkey;
ALTER TABLE lobbies ADD CONSTRAINT lobbies_stream_id_fkey
    FOREIGN KEY (stream_id) REFERENCES streams (id);
