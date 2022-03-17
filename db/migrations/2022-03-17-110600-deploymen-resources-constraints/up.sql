create or replace function deployment_resource_types__before_insert()
returns trigger as
$$
begin
    if new.env_id is null then
        if exists(select * from deployment_resource_types
        where
            env_id is null and
            deployment_kind = new.deployment_kind and
            key = new.key)
        then
            update deployment_resource_types
                set spec=new.spec
            where
                env_id is null and
                deployment_kind=new.deployment_kind and
                key=new.key;
            return null;
        end if;

        if exists(select * from deployment_resource_types
        where
            env_id is not null and
            deployment_kind = new.deployment_kind and
            key = new.key)
        then
            raise exception 'Deployment resource type with kind=% key=% already defined with env_id != null', new.deployment_kind, new.key
            using errcode = 'unique_violation';
        end if;
    else
        if exists(select * from deployment_resource_types
        where
            env_id=new.env_id and
            deployment_kind = new.deployment_kind and
            key = new.key)
        then
            update deployment_resource_types
                set spec=new.spec
            where
                env_id=new.env_id and
                deployment_kind=new.deployment_kind and
                key=new.key;
            return null;
        end if;

        if exists(select * from deployment_resource_types
        where
            env_id is null and
            deployment_kind = new.deployment_kind and
            key = new.key)
        then
            raise exception 'Deployment resource type with kind=% key=% already defined with env_id=null', new.deployment_kind, new.key
            using errcode = 'unique_violation';
        end if;
    end if;

    return new;
end;
$$
language plpgsql;

create trigger deployment_resource_types__before_insert
before insert on deployment_resource_types
for each row execute procedure deployment_resource_types__before_insert();
