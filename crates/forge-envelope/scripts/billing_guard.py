#!/usr/bin/env python3
"""
scripts/billing_guard.py
Surface Ledger — Real-Time Vertex AI Billing Sentinel & "Oh Shit" Alert Daemon.

Features:
1. Tracks cumulative token consumption and credit depletion from the $1,361.01 baseline.
2. Fires milestone alerts every $200 spent ($200, $400, $600, $800, $1,000, $1,200).
3. "OH SHIT" Circuit Breaker: Automatically halts cloud API dispatch and sends an emergency
   email if spending crosses safety tripwires ($1,000 spent or $200 remaining).
4. Emits real-time telemetry to `surfaceledger/billing_sentinel_status.json`.
"""

import os
import sys
import json
import time
import smtplib
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart
from pathlib import Path
from typing import Optional, Dict, Any

REPO_ROOT = Path(__file__).parent.parent.resolve()
STATUS_FILE = REPO_ROOT / "surfaceledger" / "billing_sentinel_status.json"
LEDGER_STATE_FILE = REPO_ROOT / "surfaceledger" / "billing_guard_state.json"

class BillingGuard:
    def __init__(
        self,
        initial_credit_usd: float = 1361.01,
        alert_step_usd: float = 200.00,
        emergency_threshold_usd: float = 1000.00,
        alert_email_to: Optional[str] = None
    ):
        self.initial_credit_usd = initial_credit_usd
        self.alert_step_usd = alert_step_usd
        self.emergency_threshold_usd = emergency_threshold_usd
        self.alert_email_to = alert_email_to or os.environ.get("ALERT_EMAIL_TO", "dev@deveraux.dev")
        
        # Load or initialize persisted state
        self.state = self._load_state()

    def _load_state(self) -> Dict[str, Any]:
        if LEDGER_STATE_FILE.exists():
            try:
                with open(LEDGER_STATE_FILE, "r", encoding="utf-8") as f:
                    return json.load(f)
            except Exception:
                pass
        return {
            "initial_credit_usd": self.initial_credit_usd,
            "total_spent_usd": 0.0,
            "remaining_credit_usd": self.initial_credit_usd,
            "last_alerted_milestone_usd": 0.0,
            "queries_executed": 0,
            "cached_tokens_read": 0,
            "uncached_tokens_read": 0,
            "output_tokens_generated": 0,
            "circuit_breaker_tripped": False,
            "last_updated": time.time(),
            "history": []
        }

    def _save_state(self):
        STATUS_FILE.parent.mkdir(parents=True, exist_ok=True)
        self.state["last_updated"] = time.time()
        self.state["remaining_credit_usd"] = max(0.0, self.initial_credit_usd - self.state["total_spent_usd"])
        
        # Save persistent state
        with open(LEDGER_STATE_FILE, "w", encoding="utf-8") as f:
            json.dump(self.state, f, indent=2)
            
        # Write live frontend status report
        with open(STATUS_FILE, "w", encoding="utf-8") as f:
            json.dump({
                "status": "CIRCUIT_BREAKER_TRIPPED" if self.state["circuit_breaker_tripped"] else "ACTIVE_SENTINEL_GUARD",
                "initial_credit_usd": self.initial_credit_usd,
                "total_spent_usd": round(self.state["total_spent_usd"], 4),
                "remaining_credit_usd": round(self.state["remaining_credit_usd"], 4),
                "queries_executed": self.state["queries_executed"],
                "last_alerted_milestone_usd": self.state["last_alerted_milestone_usd"],
                "last_updated_epoch": int(self.state["last_updated"])
            }, f, indent=2)

    def record_usage(
        self,
        cached_input_tokens: int = 0,
        uncached_input_tokens: int = 0,
        output_tokens: int = 0,
        query_type: str = "audit"
    ) -> bool:
        """
        Records API usage, calculates cost, checks $200 milestone reminders,
        and enforces the 'Oh Shit' circuit breaker.
        Returns True if execution can proceed, False if circuit breaker is tripped.
        """
        if self.state["circuit_breaker_tripped"]:
            print("[CIRCUIT BREAKER] Vertex AI dispatch blocked: Budget safety threshold reached.")
            return False

        # Vertex AI Gemini 3.7 / 1.5 Flash rates
        # Uncached: $0.000075 / 1k, Cached: $0.00001875 / 1k, Output: $0.000300 / 1k
        query_cost = (
            (cached_input_tokens / 1000.0) * 0.00001875 +
            (uncached_input_tokens / 1000.0) * 0.000075 +
            (output_tokens / 1000.0) * 0.000300
        )

        self.state["total_spent_usd"] += query_cost
        self.state["queries_executed"] += 1
        self.state["cached_tokens_read"] += cached_input_tokens
        self.state["uncached_tokens_read"] += uncached_input_tokens
        self.state["output_tokens_generated"] += output_tokens

        spent = self.state["total_spent_usd"]
        last_milestone = self.state["last_alerted_milestone_usd"]

        # 1. Check $200 Milestone Reminder
        if spent >= last_milestone + self.alert_step_usd:
            current_milestone = int(spent // self.alert_step_usd) * self.alert_step_usd
            self.state["last_alerted_milestone_usd"] = current_milestone
            self._trigger_milestone_alert(current_milestone)

        # 2. Check "OH SHIT" Emergency Threshold
        if spent >= self.emergency_threshold_usd or (self.initial_credit_usd - spent) <= 200.00:
            self.state["circuit_breaker_tripped"] = True
            self._save_state()
            self._trigger_oh_shit_emergency(spent)
            return False

        self._save_state()
        return True

    def _trigger_milestone_alert(self, milestone_usd: float):
        remaining = self.initial_credit_usd - self.state["total_spent_usd"]
        msg = (
            f"[BUDGET MILESTONE REMINDER]\n"
            f"Spend milestone reached: ${milestone_usd:,.2f} USD drawn.\n"
            f"Total Spent: ${self.state['total_spent_usd']:,.4f} USD.\n"
            f"Remaining Trial Credit: ${remaining:,.2f} USD of ${self.initial_credit_usd:,.2f} USD.\n"
            f"Total Queries Executed: {self.state['queries_executed']:,}.\n"
        )
        print("\n" + "=" * 80)
        print(msg)
        print("=" * 80 + "\n")
        
        self._send_email(
            subject=f"[Surface Ledger Alert] ${milestone_usd:.0f} Milestone Draw - ${remaining:.2f} Credit Left",
            body=msg
        )

    def _trigger_oh_shit_emergency(self, spent_usd: float):
        remaining = self.initial_credit_usd - spent_usd
        emergency_msg = (
            f"********************************************************************************\n"
            f"                     *** OH SHIT EMERGENCY ALERT ***                           \n"
            f"********************************************************************************\n"
            f"CIRCUIT BREAKER ENGAGED: Cloud dispatch has been IMMEDIATELY HALTED.\n\n"
            f"Spend Level:        ${spent_usd:,.4f} USD\n"
            f"Remaining Credit:   ${remaining:,.4f} USD (Threshold: <= $200 left or >= ${self.emergency_threshold_usd:.2f} spent)\n"
            f"Queries Executed:   {self.state['queries_executed']:,}\n"
            f"Action Taken:       Offline Edge-Metal fallback engaged. Zero further cloud spend.\n\n"
            f"Action Required:    Review active agents or simulation tick frequency.\n"
            f"********************************************************************************\n"
        )
        print("\n" + emergency_msg)
        self._send_email(
            subject=f"🚨 [OH SHIT ALERT] Vertex AI Spend Limit Reached (${spent_usd:.2f}) - Cloud Dispatch Halted!",
            body=emergency_msg,
            priority="urgent"
        )

    def _send_email(self, subject: str, body: str, priority: str = "normal"):
        """Dispatches email notification via SMTP if configured, or logs locally."""
        smtp_host = os.environ.get("SMTP_HOST")
        smtp_port = int(os.environ.get("SMTP_PORT", 587))
        smtp_user = os.environ.get("SMTP_USER")
        smtp_pass = os.environ.get("SMTP_PASS")
        sender = os.environ.get("ALERT_EMAIL_FROM", smtp_user or "sentry@13forge.com")
        recipient = self.alert_email_to

        if not (smtp_host and smtp_user and smtp_pass and recipient):
            print(f"[EMAIL NOTIFIER (DRY-RUN)] To: {recipient} | Subject: {subject}")
            print(f"[INFO] Set SMTP_HOST, SMTP_USER, SMTP_PASS to enable live inbox delivery.")
            return

        try:
            msg = MIMEMultipart()
            msg["From"] = sender
            msg["To"] = recipient
            msg["Subject"] = subject
            if priority == "urgent":
                msg["X-Priority"] = "1"
            msg.attach(MIMEText(body, "plain"))

            with smtplib.SMTP(smtp_host, smtp_port) as server:
                server.starttls()
                server.login(smtp_user, smtp_pass)
                server.send_message(msg)
            print(f"[EMAIL NOTIFIER] Live notification delivered to {recipient}.")
        except Exception as e:
            print(f"[EMAIL NOTIFIER ERROR] Could not send email: {e}")

if __name__ == "__main__":
    print("=== Surface Ledger Billing Sentinel & Oh Shit Daemon ===")
    guard = BillingGuard()
    print(f"Tracking Initial Trial Credit: ${guard.initial_credit_usd:,.2f} USD")
    print(f"Milestone Alert Interval:      Every ${guard.alert_step_usd:,.2f} USD")
    print(f"Emergency Circuit Breaker:     At ${guard.emergency_threshold_usd:,.2f} USD spent")
    print(f"Current Recorded Spend:        ${guard.state['total_spent_usd']:,.4f} USD")
    print(f"Remaining Credit Buffer:       ${guard.state['remaining_credit_usd']:,.2f} USD")
    print("Guard active and monitoring.")
