INSERT INTO tenants (slug) VALUES ('demo');


WITH t AS (SELECT id FROM tenants WHERE slug='demo')
INSERT INTO templates (tenant_id, name, content)
SELECT id, 'layout.html', $$
<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8" />
<title>{% block title %}Demo{% endblock %}</title>
</head>
<body>
<header>{% include 'partials/header.html' %}</header>
<main>{% block content %}{% endblock %}</main>
<footer>{% include 'partials/footer.html' %}</footer>
</body>
</html>
$$ FROM t;


WITH t AS (SELECT id FROM tenants WHERE slug='demo')
INSERT INTO fragments (tenant_id, name, content)
SELECT id, 'partials/header.html', '<h1>{{ site.title | default("Demo Site") }}</h1>' FROM t;


WITH t AS (SELECT id FROM tenants WHERE slug='demo')
INSERT INTO fragments (tenant_id, name, content)
SELECT id, 'partials/footer.html', '<small>© {{ now().year }} Demo</small>' FROM t;


WITH t AS (SELECT id FROM tenants WHERE slug='demo')
INSERT INTO templates (tenant_id, name, content)
SELECT id, 'pages/home.html', $$
{% extends 'layout.html' %}
{% block title %}Home — Demo{% endblock %}
{% block content %}
<p>Hello, {{ user.name | default('world') }}!</p>
{% endblock %}
$$ FROM t;


WITH t AS (SELECT id FROM tenants WHERE slug='demo')
INSERT INTO routes (tenant_id, path, template_name, data_source)
SELECT id, '/', 'pages/home.html', '{"provider":"static","payload":{"user":{"name":"Ari"}}}'::jsonb FROM t;