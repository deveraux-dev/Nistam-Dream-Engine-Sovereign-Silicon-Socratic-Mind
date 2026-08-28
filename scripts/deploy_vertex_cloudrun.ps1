# PowerShell Deployment Script for Surface Ledger on Vertex AI & GCP
$ErrorActionPreference = "Stop"

$GCP_PROJECT_ID = if ($env:GCP_PROJECT_ID) { $env:GCP_PROJECT_ID } else { (gcloud config get-value project 2>$null).Trim() }
$GCP_REGION = if ($env:GCP_REGION) { $env:GCP_REGION } else { "northamerica-northeast1" }
$SERVICE_ACCOUNT_NAME = "surface-ledger-sentry"
$BUCKET_NAME = "$GCP_PROJECT_ID-s13-inbox"

Write-Host "=== Deploying Surface Ledger Engine to $GCP_PROJECT_ID ($GCP_REGION) ===" -ForegroundColor Cyan

# 1. Enable GCP Services
Write-Host "[1/4] Enabling GCP APIs..." -ForegroundColor Yellow
gcloud services enable `
    aiplatform.googleapis.com `
    run.googleapis.com `
    firestore.googleapis.com `
    storage.googleapis.com `
    artifactregistry.googleapis.com `
    cloudbuild.googleapis.com

# 2. Service Account
Write-Host "[2/4] Configuring IAM Service Account..." -ForegroundColor Yellow
gcloud iam service-accounts create $SERVICE_ACCOUNT_NAME --display-name="Surface Ledger Sentry Engine" 2>$null

$roles = @("roles/aiplatform.user", "roles/datastore.user", "roles/storage.objectAdmin")
foreach ($role in $roles) {
    gcloud projects add-iam-policy-binding $GCP_PROJECT_ID `
        --member="serviceAccount:${SERVICE_ACCOUNT_NAME}@${GCP_PROJECT_ID}.iam.gserviceaccount.com" `
        --role=$role --quiet
}

# 3. Bucket
Write-Host "[3/4] Setting up GCS Inbox Bucket..." -ForegroundColor Yellow
gcloud storage buckets create "gs://$BUCKET_NAME" --location=$GCP_REGION 2>$null

# 4. Firestore
Write-Host "[4/4] Checking Firestore native database..." -ForegroundColor Yellow
gcloud firestore databases create --location=$GCP_REGION --type=firestore-native 2>$null

Write-Host "=== GCP provisioning complete for $GCP_PROJECT_ID ===" -ForegroundColor Green
Write-Host "APIs, service account, GCS inbox bucket, and Firestore are ready." -ForegroundColor Green
Write-Host "Run the agent against them with:" -ForegroundColor Cyan
Write-Host "  pip install -r crates/forge-envelope/requirements.txt" -ForegroundColor Cyan
Write-Host "  `$env:GCP_PROJECT_ID='$GCP_PROJECT_ID'; `$env:GCS_INBOX_BUCKET='$BUCKET_NAME'" -ForegroundColor Cyan
Write-Host "  python crates/forge-envelope/scripts/agent_loop.py" -ForegroundColor Cyan
