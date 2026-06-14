-- The API's websocket feed listens on the generic `db_notifications` channel,
-- which is fed by `notify_trigger()` (attached to every table). A previous
-- migration added `env_id` to `notify_specific_trigger_name()` instead, which
-- only feeds the per-table channels consumed by the backend services -- so the
-- websocket events reaching the frontend carried no `env_id`, and per-env
-- subscription/permission filtering dropped every env-scoped event.
--
-- This enriches the generic `notify_trigger()` with the environment of the
-- changed row, resolved per table. Because the trigger uses OLD on DELETE, the
-- environment is available even for deletions.
--
--   deployments          -> k8s_clusters.env_id (via cluster_id)
--   deployment_tasks     -> k8s_clusters.env_id (via cluster_id)
--   deployment_resources -> deployments -> k8s_clusters.env_id
--   secrets              -> secrets.env_id
--   env_user_permissions -> env_user_permissions.env_id
--   deployment_permissions -> deployment_permissions.env_id
--   envs                 -> envs.id (the environment itself)
-- All other tables are not environment-scoped and carry a null env_id.

CREATE OR REPLACE FUNCTION notify_trigger() RETURNS trigger AS $trigger$
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
  WHEN 'secrets' THEN
     v_env_id := rec.env_id;
  WHEN 'env_user_permissions' THEN
     v_env_id := rec.env_id;
  WHEN 'deployment_permissions' THEN
     v_env_id := rec.env_id;
  WHEN 'envs' THEN
     v_env_id := rec.id;
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
  PERFORM pg_notify('db_notifications', payload);

  RETURN rec;
END;
$trigger$ LANGUAGE plpgsql;
