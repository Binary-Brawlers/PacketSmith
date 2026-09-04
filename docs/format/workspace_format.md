# PacketSmith Workspace File Format Specification

## Overview

PacketSmith workspaces are stored on disk as human-readable, Git-friendly directories.
Every resource is saved as an individual YAML file, enabling meaningful Git diffs, effortless branching, and clean pull request reviews.

---

## Directory Structure

```text
my-api-workspace/
├── packetsmith.yaml                # Workspace root manifest
├── collections/
│   ├── authentication/
│   │   ├── authentication.col.yaml # Collection root metadata
│   │   ├── login.req.yaml          # Request document
│   │   ├── refresh-token.req.yaml  # Request document
│   │   └── oauth/
│   │       ├── oauth.folder.yaml   # Nested folder metadata
│   │       └── authorize.req.yaml  # Request inside folder
│   └── users/
│       ├── users.col.yaml
│       └── get-profile.req.yaml
├── environments/
│   ├── local.env.yaml              # Local development variables
│   ├── staging.env.yaml            # Staging environment
│   └── production.env.yaml         # Production environment
└── specs/
    └── petstore.openapi.yaml       # Imported or linked API specifications
```

---

## File Naming Conventions

All resource filenames follow the `<slug>.<type>.yaml` format:
- **Root manifest:** `packetsmith.yaml`
- **Collections:** `<slug>.col.yaml`
- **Folders:** `<slug>.folder.yaml`
- **Requests:** `<slug>.req.yaml`
- **Environments:** `<slug>.env.yaml`

Slugs are derived from the human-readable resource name by converting to lowercase, replacing non-alphanumeric characters with single hyphens, and trimming edge hyphens.

---

## Resource Schemas

### 1. `packetsmith.yaml` (Workspace Manifest)

```yaml
version: "1.0.0"
id: "9b1deb4d-3b7d-4bad-9bdd-2b0d7b3dcb6d"
name: "Production Services"
description: "Core banking and user authentication microservices"
resource_roots:
  - "collections"
  - "environments"
  - "specs"
active_environment: "staging"
```

### 2. `<slug>.col.yaml` (Collection)

```yaml
id: "b21e8a09-64c2-48df-9f37-12c8b74a3f12"
name: "Authentication"
description: "Identity and session endpoints"
auth:
  type: "bearer"
  token_secret_ref: "AUTH_TOKEN"
variables:
  - key: "auth_prefix"
    value: "/v1/auth"
    is_secret: false
    enabled: true
scripts:
  pre_request: |
    console.log("Preparing auth request");
  post_response: null
item_order:
  - "4fa3e912-..."
  - "7c29a103-..."
created_at: "2026-09-04T12:00:00Z"
updated_at: "2026-09-04T12:00:00Z"
```

### 3. `<slug>.req.yaml` (Request)

```yaml
id: "4fa3e912-70b1-4c12-87ba-9642e12891bb"
name: "Login"
tags: ["auth", "public"]
protocol:
  type: "http"
  method: "POST"
  url: "{{base_url}}/login"
auth:
  type: "none"
scripts:
  pre_request: null
  post_response: |
    if (response.status === 200) {
      env.set("token", response.json().access_token);
    }
settings:
  follow_redirects: true
  max_redirects: 10
  verify_ssl: true
  timeout_ms: 30000
examples: []
created_at: "2026-09-04T12:00:00Z"
updated_at: "2026-09-04T12:00:00Z"
```

### 4. `<slug>.env.yaml` (Environment)

```yaml
id: "e5192bf8-42f1-46ab-829d-648b29c95211"
name: "Staging"
description: "Staging Kubernetes cluster endpoints"
variables:
  - key: "base_url"
    value: "https://staging.api.example.com"
    is_secret: false
    enabled: true
  - key: "api_key"
    value: "{{vault:STAGING_API_KEY}}"
    is_secret: true
    enabled: true
created_at: "2026-09-04T12:00:00Z"
updated_at: "2026-09-04T12:00:00Z"
```

---

## Secret Exclusion Rule

Values marked with `is_secret: true` store references (`{{vault:SECRET_NAME}}`) rather than raw plaintext secrets when committed to version control. The actual secret values are held in the local OS keychain or encrypted vault.
