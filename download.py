#!/usr/bin/env python3
"""
downloads activities (FIT files only)
health metrics and training metrics are commented out
"""

import argparse
import json
import os
import sys
import zipfile
from datetime import datetime, timedelta
from getpass import getpass
from io import BytesIO
from pathlib import Path

from dotenv import load_dotenv
from garminconnect import Garmin, GarminConnectAuthenticationError
from tqdm import tqdm


def setup_directories():
    Path("data/raw").mkdir(parents=True, exist_ok=True)


def authenticate(email: str, password: str, tokenstore: str = "~/.garminconnect") -> Garmin:
    tokenstore = os.path.expanduser(tokenstore)

    try:
        print(f"Attempting to use stored tokens from: {tokenstore}")
        client = Garmin()
        client.login(tokenstore)
        print("Successfully authenticated using stored tokens")
        return client
    except Exception:
        print("No valid tokens found, requesting credentials...")

    try:
        client = Garmin(email=email, password=password, return_on_mfa=True)
        result1, result2 = client.login()

        if result1 == "needs_mfa":
            print("Multi-factor authentication required")
            mfa_code = input("Enter MFA code: ")
            client.resume_login(result2, mfa_code)
            print("MFA authentication successful")

        client.garth.dump(tokenstore)
        print(f"Authentication tokens saved to: {tokenstore}")
        return client

    except GarminConnectAuthenticationError as e:
        print(f"Authentication failed: {e}", file=sys.stderr)
        return None
    except Exception as e:
        print(f"Unexpected error during authentication: {e}", file=sys.stderr)
        return None


def save_json(data, filepath):
    if data:
        with open(filepath, "w") as f:
            json.dump(data, f, indent=2)
        return True
    return False


def download_activities(client: Garmin, start_date: str, end_date: str, example_mode: bool = False):
    print(f"\n[ACTIVITIES] Fetching from {start_date} to {end_date}...")

    try:
        activities = client.get_activities_by_date(start_date, end_date)

        if not activities:
            print("  No activities found")
            return

        print(f"  Found {len(activities)} activities")

        for activity in tqdm(activities, desc="Activities", unit="activity"):
            activity_id = activity["activityId"]
            activity_name = activity.get("activityName", "Unnamed")

            # Use "example" naming in example mode, otherwise use activity_id
            if example_mode:
                json_path = "example.json"
                fit_path = "example.fit"
            else:
                json_path = f"data/raw/activities_{activity_id}.json"
                fit_path = f"data/raw/activities_{activity_id}.fit"

            # Save JSON metadata
            if not os.path.exists(json_path):
                save_json(activity, json_path)

            # Download FIT file
            if not os.path.exists(fit_path):
                try:
                    fit_data = client.download_activity(
                        activity_id,
                        dl_fmt=client.ActivityDownloadFormat.ORIGINAL
                    )

                    # Check if the data is a ZIP file (Garmin sometimes returns zipped FIT files)
                    if fit_data[:2] == b'PK':  # ZIP file signature
                        with zipfile.ZipFile(BytesIO(fit_data)) as zip_file:
                            # Get the first .fit file from the archive
                            fit_files = [name for name in zip_file.namelist() if name.lower().endswith('.fit')]
                            if fit_files:
                                fit_data = zip_file.read(fit_files[0])

                    with open(fit_path, "wb") as f:
                        f.write(fit_data)
                except Exception as e:
                    tqdm.write(f"  ✗ {activity_name} (FIT failed: {e})")

    except Exception as e:
        print(f"  ✗ Error: {e}")


# def download_health_metrics(client: Garmin, date: str):
#     metrics = [
#         ("stats", lambda: client.get_stats(date)),
#         ("heart_rate", lambda: client.get_heart_rates(date)),
#         ("sleep", lambda: client.get_sleep_data(date)),
#         ("stress", lambda: client.get_all_day_stress(date)),
#         ("steps", lambda: client.get_steps_data(date)),
#         ("hrv", lambda: client.get_hrv_data(date)),
#         ("spo2", lambda: client.get_spo2_data(date)),
#         ("respiration", lambda: client.get_respiration_data(date)),
#         ("hydration", lambda: client.get_hydration_data(date)),
#         ("intensity_minutes", lambda: client.get_intensity_minutes_data(date)),
#     ]
#
#     for metric_name, fetch_func in tqdm(metrics, desc=f"Health {date}", unit="metric", leave=False):
#         filepath = f"data/raw/health_{metric_name}_{date}.json"
#
#         if os.path.exists(filepath):
#             continue
#
#         try:
#             data = fetch_func()
#             if not save_json(data, filepath):
#                 tqdm.write(f"  ⊘ {metric_name} (no data)")
#         except Exception as e:
#             tqdm.write(f"  ✗ {metric_name} ({type(e).__name__})")


# def download_training_metrics(client: Garmin, date: str):
#     metrics = [
#         ("readiness", lambda: client.get_training_readiness(date)),
#         ("status", lambda: client.get_training_status(date)),
#         ("max_metrics", lambda: client.get_max_metrics(date)),
#     ]
#
#     for metric_name, fetch_func in tqdm(metrics, desc=f"Training {date}", unit="metric", leave=False):
#         filepath = f"data/raw/training_{metric_name}_{date}.json"
#
#         if os.path.exists(filepath):
#             continue
#
#         try:
#             data = fetch_func()
#             if not save_json(data, filepath):
#                 tqdm.write(f"  ⊘ {metric_name} (no data)")
#         except Exception as e:
#             tqdm.write(f"  ✗ {metric_name} ({type(e).__name__})")


def parse_date(date_str: str) -> str:
    try:
        datetime.strptime(date_str, "%Y-%m-%d")
        return date_str
    except ValueError:
        raise argparse.ArgumentTypeError(
            f"Invalid date format: {date_str}. Expected YYYY-MM-DD"
        )


def main():
    parser = argparse.ArgumentParser(
        description="Download Garmin Connect activities (FIT files only)"
    )
    parser.add_argument(
        "--start-date",
        type=parse_date,
        help="Start date (YYYY-MM-DD). Defaults to 7 days ago"
    )
    parser.add_argument(
        "--end-date",
        type=parse_date,
        help="End date (YYYY-MM-DD). Defaults to today"
    )
    parser.add_argument(
        "--tokenstore",
        default="~/.garminconnect",
        help="Path to token storage directory (default: ~/.garminconnect)"
    )
    parser.add_argument(
        "--example",
        action="store_true",
        help="Download cycling activity from September 14th, 2025 for testing"
    )

    args = parser.parse_args()

    if args.example:
        args.start_date = "2025-09-14"
        args.end_date = "2025-09-14"
        print("[EXAMPLE MODE] Downloading activity from 2025-09-14")

    load_dotenv()

    email = os.getenv("EMAIL")
    password = os.getenv("PASSWORD")

    if not email:
        email = input("Garmin Connect email: ")
    if not password:
        password = getpass("Garmin Connect password: ")

    if not args.end_date:
        end_date = datetime.now()
    else:
        end_date = datetime.strptime(args.end_date, "%Y-%m-%d")

    if not args.start_date:
        start_date = end_date - timedelta(days=7)
    else:
        start_date = datetime.strptime(args.start_date, "%Y-%m-%d")

    setup_directories()

    client = authenticate(email, password, args.tokenstore)
    if not client:
        sys.exit(1)

    download_activities(
        client,
        start_date.strftime("%Y-%m-%d"),
        end_date.strftime("%Y-%m-%d"),
        example_mode=args.example
    )

    # Commented out: Only downloading activities.fit files
    # if not args.example:
    #     num_days = (end_date - start_date).days + 1
    #     dates = [start_date + timedelta(days=i) for i in range(num_days)]
    #
    #     for date in tqdm(dates, desc="Daily metrics", unit="day"):
    #         date_str = date.strftime("%Y-%m-%d")
    #         download_health_metrics(client, date_str)
    #         download_training_metrics(client, date_str)

    print("\nDownload complete!")


if __name__ == "__main__":
    main()
