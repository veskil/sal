from pathlib import Path
import datetime as dt
import os
import colorama
import json
import pandas as pd
from zoneinfo import ZoneInfo

ROOT = Path(__file__).parent
LOG_DIR = ROOT / "logs/"
STATS_DIR = ROOT / "stats/"
USER_JSON_FILE = ROOT / "users.json"

GREETING = """\
Velkommen til Sal <3
Tæpp NTNU-kortet ditt på kortleseren for å registrere ankomst eller avreise.
"""

PROMPT = """\
enter     : nullstill skjerm
i + enter : vis instruksjoner
s + enter : vis statistikk
u + enter : sett nytt brukernavn
q + enter : avslutt programmet
> \
"""

INSTRUCTIONS = """\
INSTRUKSJONER:

Tæpp NTNU-kortet ditt på kortleseren for å registrere ankomst eller avreise.
Det er kun første og siste tæpp for dagen* som får noe å si for statistikken,
de tolkes som ankomst og avreise.
Alle tæpp mellom første og siste har ingenting å si.
Det er altså trygt å tæppe en gang for mye.

Man vil snart få servert statistikk som antall dager streak på sal,
gjennomsnittlig oppmøtetid, etc., men dette implementeres senere.
Gjerne begynn å loggføre ankomst og avreise allerede nå likevel!

NB: KORTET INNEHOLDER TO NUMMER. SJEKK AT DU REGISRERES RIKTIG.

*Dagen begynner 05:00 og slutter 04:59 neste dag.
"""

def highlight(str_to_highlight: str) -> str:
    return colorama.Fore.GREEN + str_to_highlight + colorama.Fore.RESET


def update_username(users: dict[str, str], card_num: str, new_username: str):
    users[card_num] = new_username
    with open(USER_JSON_FILE, "w") as f:
        json.dump(users, f, indent=0)


def clear_and_print(message):
    os.system("clear")
    print(message)


def log_entry(card_num: str, timestamp: dt.datetime) -> None:
    log_message = f"{timestamp.isoformat()},{card_num}\n"
    log_file = LOG_DIR / (timestamp.strftime("%Y%m%d") + ".log")
    with open(log_file, "a") as file:
        file.write(log_message)


def update_stats(user_stat_df: pd.DataFrame, timestamp: dt.datetime):
    local_datetime = timestamp.astimezone(ZoneInfo("Europe/Oslo"))
    local_time = local_datetime.time()
    effective_date = (local_datetime - dt.timedelta(hours=5)).date()
    last_entry_effective_date = user_stat_df.iloc[-1].loc["date"]
    if last_entry_effective_date == effective_date:
        user_stat_df.iloc[-1].loc["last_tap_time"] = local_time
    else:
        num_days_since_last_entry = (effective_date - last_entry_effective_date).days
        num_weekdays_since_last_entry = sum([((last_entry_effective_date + dt.timedelta(days=x)).isoweekday() < 6)
                                              for x in range(num_days_since_last_entry)])
        current_streak = (user_stat_df.iloc[-1].loc["current_streak"] + 1) if num_weekdays_since_last_entry <= 1 else 1
        user_stat_df.iloc[len(user_stat_df.index)] = {
            "date":effective_date,
            "first_tap_time":local_time,
            "last_tap_time":local_time,
            "current_streak":current_streak
        }


def main():
    with open(USER_JSON_FILE, "r") as f:
        usernames: dict[str, str] = json.load(f)

    # Load user statistics
    user_stat_dfs: dict[str, pd.DataFrame] = dict()
    for username in usernames.values():
        user_stat_path = STATS_DIR / f"{username}.feather"
        if user_stat_path.exists():
            user_stat_dfs[username] = pd.read_feather(user_stat_path)
        else:
            user_stat_dfs[username] = pd.DataFrame(columns=["date", "first_tap_time", "last_tap_time", "current_streak"])
            user_stat_dfs[username].to_feather(user_stat_path)

    user_input = "dummy"
    logged_in_card_num = None
    logged_in_username = None
    clear_and_print(GREETING)

    while user_input.lower() not in ["q", "quit"]:
        user_input = input(PROMPT)

        # Log card read
        if user_input.isnumeric() and len(user_input) == 10:
            logged_in_card_num = user_input

            # Find username
            if logged_in_card_num not in usernames:
                update_username(usernames, logged_in_card_num, logged_in_card_num)
            logged_in_username = usernames[logged_in_card_num]

            # Print hello message
            clear_and_print("Velkommen " + highlight(usernames[logged_in_card_num]) + "!")
            if logged_in_card_num == logged_in_username:
                print("Du kan sette et brukernavn ved å trykke 'u' etterfulgt av 'enter'.")

            # Log entry and update stats, print streak
            now = dt.datetime.now(tz=dt.UTC)
            log_entry(logged_in_card_num, now)
            update_stats(user_stat_dfs[logged_in_username], now)
            user_stat_dfs[username].to_feather(user_stat_path)
            print(("🔥" * user_stat_dfs[username].iloc[-1].loc["current_streak"]) + "\n")

        match user_input:
            # Log out / reset screen
            case "":
                logged_in_card_num = None
                logged_in_username = None
                clear_and_print(GREETING)

            # Show instructions
            case "i":
                clear_and_print(INSTRUCTIONS)

            # Show statistics
            case "s":
                clear_and_print("Brukerstatistikk er ikke implementert ennå :(\n")

            # Change username
            case "u":
                if logged_in_card_num is None:
                    clear_and_print("Må tæppe kort først!\n")
                else:
                    new_username = input(f"Skriv inn brukernavn for kort med nummer {highlight(logged_in_card_num)}: ")
                    update_username(usernames, logged_in_card_num, new_username)
                    clear_and_print(f"Brukernavn {highlight(new_username)} registrert for kort {highlight(logged_in_card_num)}")


if __name__ == "__main__":
    main()
