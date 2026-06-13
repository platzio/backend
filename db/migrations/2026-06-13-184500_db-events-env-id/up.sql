-- Enrich database change notifications with the environment of the changed
-- row. The API uses this to forward each websocket event only to clients that
-- are permitted to see that environment, without a per-event lookup. Because
-- the environment is resolved inside the trigger (from OLD on DELETE), it is
-- available even for deletions, where the row no longer exists afterwards.
--
-- The environment is not a direct column on the notified tables, so it is
-- resolved through the cluster:
--   deployments          -> k8s_clusters.env_id (via cluster_id)
--   deployment_tasks     -> k8s_clusters.env_id (via cluster_id)
--   deployment_resources -> deployments -> k8s_clusters.env_id
-- Global tables (e.g. helm_tag_formats) carry a null env_id.

CREATE OR REPLACE FUNCTION notify_specific_trigger_name() RETURNS trigger AS $trigger$
DECLARE
  rec RECORD;
  payload TEXT;
  column_name TEXT;
  column_value TEXT;
  payload_items TEXT[];
  v_env_id UUID;
  env_id_json TEXT;
BEGIN
  -- Set record row depending on operation
  CASE TG_OP
  WHEN 'INSERT', 'UPDATE' THEN
     rec := NEW;
  WHEN 'DELETE' THEN
     rec := OLD;
  ELSE
     RAISE EXCEPTION 'Unknown TG_OP: "%". Should not occur!', TG_OP;
  END CASE;

  -- Resolve the environment of the changed row, where applicable.
  v_env_id := NULL;
  CASE TG_TABLE_NAME
  WHEN 'deployments' THEN
     SELECT k.env_id INTO v_env_id FROM k8s_clusters k WHERE k.id = rec.cluster_id;
  WHEN 'deployment_tasks' THEN
     SELECT k.env_id INTO v_env_id FROM k8s_clusters k WHERE k.id = rec.cluster_id;
  WHEN 'deployment_resources' THEN
     SELECT k.env_id INTO v_env_id
       FROM deployments d
       JOIN k8s_clusters k ON k.id = d.cluster_id
       WHERE d.id = rec.deployment_id;
  ELSE
     v_env_id := NULL;
  END CASE;

  IF v_env_id IS NULL THEN
     env_id_json := 'null';
  ELSE
     env_id_json := '"' || v_env_id::TEXT || '"';
  END IF;

  -- Get required fields
  FOREACH column_name IN ARRAY TG_ARGV LOOP
    EXECUTE format('SELECT $1.%I::TEXT', column_name)
    INTO column_value
    USING rec;
    payload_items := array_append(payload_items, '"' || replace(column_name, '"', '\"') || '":"' || replace(column_value, '"', '\"') || '"');
  END LOOP;

  -- Build the payload
  payload := ''
              || '{'
              || '"timestamp":"' || CURRENT_TIMESTAMP                    || '",'
              || '"operation":"' || TG_OP                                || '",'
              || '"schema":"'    || TG_TABLE_SCHEMA                      || '",'
              || '"table":"'     || TG_TABLE_NAME                        || '",'
              || '"env_id":'     || env_id_json                          || ','
              || '"data":{'      || array_to_string(payload_items, ',')  || '}'
              || '}';

  -- Notify the channel
  PERFORM pg_notify(format('db_%I_notifications',TG_TABLE_NAME), payload);
  RETURN rec;
END;
$trigger$ LANGUAGE plpgsql;
