-- migrations/20251022_000001_init.sql
CREATE TABLE tenants (
id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
slug TEXT UNIQUE NOT NULL, -- e.g. "acme"
domain TEXT UNIQUE, -- e.g. "acme.com" (optional)
created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);


CREATE TABLE routes (
id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
path TEXT NOT NULL, -- "/" | "/products/:handle" | etc
template_name TEXT NOT NULL, -- e.g. "pages/home.html"
data_source JSONB NOT NULL DEFAULT '{}'::jsonb, -- config for provider
is_published BOOLEAN NOT NULL DEFAULT TRUE,
updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
UNIQUE (tenant_id, path)
);


CREATE TABLE templates (
id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
name TEXT NOT NULL, -- "pages/home.html"
content TEXT NOT NULL, -- raw MiniJinja text
updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
UNIQUE (tenant_id, name)
);


CREATE TABLE fragments (
id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
name TEXT NOT NULL, -- "partials/header.html"
content TEXT NOT NULL,
updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
UNIQUE (tenant_id, name)
);