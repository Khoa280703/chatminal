# Stitch MCP Server Access Report

Date: 2026-03-10  
Task: Retrieve Pro Terminal Session Manager screen design  
Project ID: 17566574904910358980  
Screen ID: 558d2fe2613c471aaec48dd29f8d7e86  
Screen Name: Pro Terminal Session Manager  

## Status

**BLOCKED** - Authentication required to access Stitch API

## Findings

### 1. Stitch MCP API is Accessible
- Endpoint: `https://stitch.googleapis.com/mcp`
- API responds with list of available tools
- No network/connectivity issues

### 2. Available Tools
The Stitch API provides the following tools:
- `create_project` - Create new projects
- `get_project` - Get project details
- `list_projects` - List projects
- `list_screens` - List screens in project
- `get_screen` - Get screen details
- `generate_screen_from_text` - Generate screens from prompts
- `edit_screens` - Edit screen designs
- `generate_variants` - Generate design variants

### 3. Authentication Issue
All tool calls return error:
```
"Request is missing required authentication credential. Expected OAuth 2 access token, login cookie or other valid authentication credential."
```

### 4. Authentication Methods (Attempted)

#### Method 1: Environment Variables
- **Status**: Failed
- **Tried**: `GOOGLE_CLOUD_PROJECT=17566574904910358980`
- **Result**: Stitch MCP server requires `gcloud` and credentials

#### Method 2: Direct HTTPS API Call
- **Status**: Failed
- **Reason**: Missing OAuth 2.0 access token in request headers

#### Method 3: Gemini CLI via MCP
- **Status**: Failed
- **Reason**: Stitch MCP server needs `gcloud auth application-default login` first

### 5. Authentication Requirements

Per stitch-mcp npm package README, authentication requires:

1. **Google Cloud Project Setup**
   ```bash
   gcloud auth login
   gcloud config set project YOUR_PROJECT_ID
   gcloud auth application-default set-quota-project YOUR_PROJECT_ID
   gcloud beta services mcp enable stitch.googleapis.com
   ```

2. **Application Default Credentials**
   ```bash
   gcloud auth application-default login
   ```

3. **Environment Variable** (for CLI/MCP)
   - `GOOGLE_CLOUD_PROJECT`: Project ID (17566574904910358980)

4. **OAuth 2.0 Access Token** (for direct API calls)
   - Needed in request headers when calling HTTPS API directly

## What's Blocking

- **gcloud CLI**: Not installed on system (`gcloud` command not found)
- **Google Credentials**: No application-default credentials configured
- **API Key**: No Stitch/Google API key provided in environment
- **Service Account**: No service account JSON file available

## Next Steps Needed

To retrieve the screen design, one of these must be provided:

1. **Google Cloud Authentication**
   - Install `gcloud` CLI
   - Run `gcloud auth application-default login`
   - Ensure project has Stitch API enabled

2. **API Key/Token**
   - Provide OAuth 2.0 access token
   - Or provide API key (if available for Stitch API)

3. **Service Account Credentials**
   - Provide service account JSON file
   - Set `GOOGLE_APPLICATION_CREDENTIALS` environment variable

## Unresolved Questions

1. Is there a pre-configured Google Cloud project for this workspace?
2. Should we use user authentication or service account?
3. Is the Stitch API key available in project secrets/vault?
4. Can we use an existing OAuth token from another service?

