CREATE FUNCTION notify_specific_trigger_name() RETURNS trigger AS $trigger$
DECLARE
  rec RECORD;
  payload TEXT;
  column_name TEXT;
  column_value TEXT;
  payload_items TEXT[];
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
              || '"data":{'      || array_to_string(payload_items, ',')  || '}'
              || '}';

  -- Notify the channel
  PERFORM pg_notify(format('db_%I_notifications',TG_TABLE_NAME), payload);
  RETURN rec;
END;
$trigger$ LANGUAGE plpgsql;



create trigger notify_deployment_tasks_changes after insert or update or delete on deployment_tasks
for each row execute procedure notify_specific_trigger_name('id');

create trigger notify_deployments_changes after insert or update or delete on deployments
for each row execute procedure notify_specific_trigger_name('id');

create trigger notify_deployment_resources_changes after insert or update or delete on deployment_resources
for each row execute procedure notify_specific_trigger_name('id');

create trigger notify_helm_tag_formats_changes after insert or update or delete on helm_tag_formats
for each row execute procedure notify_specific_trigger_name('id');
