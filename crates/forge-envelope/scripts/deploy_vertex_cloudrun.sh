#!/usr/bin/env bash
# Deploy Surface Ledger Agent & Sentry Engine to Google Cloud Run and Vertex AI
set -euo pipefail

export GCP_PROJECT_ID="${GCP_PROJECT_ID:-$(gcloud config get-value project)}"
export GCP_REGION="${GCP_REGION:-northamerica-northeast1}"
export SERVICE_ACCOUNT_NAME="surface-ledger-sentry"
export BUCKET_NAME="${GCP_PROJECT_ID}-s13-inbox"

echo "=== Deploying Surface Ledger Engine to ${GCP_PROJECT_ID} (${GCP_REGION}) ==="

# 1. Enable Required GCP APIs
echo "[1/5] Enabling GCP APIs..."
gcloud services enable \
    aiplatform.googleapis.com \
    run.googleapis.com \
    firestore.googleapis.com \
    storage.googleapis.com \
    artifactregistry.googleapis.com \
    cloudbuild.googleapis.com

# 2. Provision Service Account
echo "[2/5] Configuring IAM Service Account..."
if ! gcloud iam service-accounts describe "${SERVICE_ACCOUNT_NAME}@${GCP_PROJECT_ID}.iam.gserviceaccount.com" &>/dev/null; then
    gcloud iam service-accounts create "$SERVICE_ACCOUNT_NAME" \
        --display-name="Surface Ledger Sentry Engine"
fi

for role in roles/aiplatform.user roles/datastore.user roles/storage.objectAdmin; do
    gcloud projects add-iam-policy-binding "$GCP_PROJECT_ID" \
        --member="serviceAccount:${SERVICE_ACCOUNT_NAME}@${GCP_PROJECT_ID}.iam.gserviceaccount.com" \
        --role="$role" --quiet
done

# 3. Create Storage Bucket
echo "[3/5] Setting up GCS Inbox Bucket..."
if ! gcloud storage buckets describe "gs://${BUCKET_NAME}" &>/dev/null; then
    gcloud storage buckets create "gs://${BUCKET_NAME}" --location="$GCP_REGION"
fi

# 4. Initialize Firestore
echo "[4/5] Checking Firestore native database..."
gcloud firestore databases create --location="$GCP_REGION" --type=firestore-native 2>/dev/null || true

# 5. Build and Deploy Container to Cloud Run
echo "[5/5] Building and Deploying to Cloud Run..."
gcloud builds submit --tag "gcr.io/${GCP_PROJECT_ID}/surface-ledger-agent:latest" .

gcloud run deploy surface-ledger-sentry \
    --image "gcr.io/${GCP_PROJECT_ID}/surface-ledger-agent:latest" \
    --region "$GCP_REGION" \
    --service-account "${SERVICE_ACCOUNT_NAME}@${GCP_PROJECT_ID}.iam.gserviceaccount.com" \
    --memory 4Gi \
    --cpu 2 \
    --min-instances 1 \
    --max-instances 10 \
    --set-env-vars "GEMINI_API_KEY=${GEMINI_API_KEY:-},GCP_PROJECT_ID=${GCP_PROJECT_ID},GCS_INBOX_BUCKET=${BUCKET_NAME},FORGE_ENVELOPE_BIN=/usr/local/bin/attest"

echo "=== Surface Ledger Agent Deployed Successfully ==="
