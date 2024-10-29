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

class User:
    def __init__(self, card_num: str, usernames: dict[str, str]):
        self.card_num = card_num

        if card_num not in usernames.keys():
            update_username(usernames, card_num, card_num)
        self.username = usernames[card_num]

        self.stat_path = STATS_DIR / f"{self.username}.feather"
        if self.stat_path.exists():
            self.stat_df = pd.read_feather(self.stat_path)
        else:
            self.stat_df = pd.DataFrame(columns=["date", "first_tap_time", "last_tap_time", "current_streak"])
            self.stat_df.to_feather(self.stat_path)


def highlight(str_to_highlight: str) -> str:
    return colorama.Fore.GREEN + str_to_highlight + colorama.Fore.RESET


def update_username(usernames: dict[str, str], card_num: str, new_username: str):
    usernames[card_num] = new_username
    with open(USER_JSON_FILE, "w") as f:
        json.dump(usernames, f, indent=0)


def clear_and_print(message):
    os.system("clear")
    print(message)


def log_entry(user: User, timestamp: dt.datetime) -> None:
    log_message = f"{timestamp.isoformat()},{user.card_num}\n"
    log_file = LOG_DIR / (timestamp.strftime("%Y%m%d") + ".log")
    with open(log_file, "a") as file:
        file.write(log_message)


def update_stats(user: User, timestamp: dt.datetime):
    local_datetime = timestamp.astimezone(ZoneInfo("Europe/Oslo"))
    effective_date = (local_datetime - dt.timedelta(hours=5)).date()
    len_df = len(user.stat_df.index)
    if len_df == 0:
        user.stat_df.loc[0] = {
            "date":effective_date,
            "first_tap_time":local_datetime,
            "last_tap_time":local_datetime,
            "hours":0,
            "current_streak":1
        }
    elif user.stat_df.loc[len_df - 1, "date"] == effective_date:
        user.stat_df.loc[len_df - 1, "last_tap_time"] = local_datetime
        user.stat_df.loc[len_df - 1, "hours"] = (user.stat_df.loc[len_df - 1, "last_tap_time"] - user.stat_df.loc[len_df - 1, "first_tap_time"]).total_seconds() / 3600
    else:
        last_entry_effective_date = user.stat_df.loc[len_df - 1, "date"]
        num_days_since_last_entry = (effective_date - last_entry_effective_date).days
        num_weekdays_since_last_entry = sum([((last_entry_effective_date + dt.timedelta(days=x)).isoweekday() < 6)
                                              for x in range(num_days_since_last_entry)])
        current_streak = (user.stat_df.loc[len_df - 1, "current_streak"] + 1) if num_weekdays_since_last_entry <= 1 else 1
        user.stat_df.loc[len(user.stat_df.index)] = {
            "date":effective_date,
            "first_tap_time":local_datetime,
            "last_tap_time":local_datetime,
            "hours":0,
            "current_streak":current_streak
        }
    user.stat_df.to_feather(user.stat_path)


def get_statistics_message(user: User) -> str:
    today = user.stat_df.loc[user.stat_df.index[-1], "date"]
    current_streak = user.stat_df.iloc[-1]["current_streak"]
    num_days_total = len(user.stat_df)
    num_days_last_30 = len(user.stat_df[today - user.stat_df["date"] < dt.timedelta(days=30)])
    num_days_last_7 = len(user.stat_df[today - user.stat_df["date"] < dt.timedelta(days=7)])
    longest_day_date = user.stat_df.loc[user.stat_df["hours"].argmax(), "date"]
    longest_day_duration = user.stat_df["hours"].max()
    earliest_arrival = user.stat_df.loc[user.stat_df["first_tap_time"].apply(lambda t: (t - dt.timedelta(hours=5)).time()).argmin(), "first_tap_time"]
    latest_departure = user.stat_df.loc[user.stat_df["last_tap_time"].apply(lambda t: (t - dt.timedelta(hours=5)).time()).argmax(), "last_tap_time"]

    message = (
        f"Nåværende streak: {current_streak}\n"
        f"Antall oppmøtedager totalt: {num_days_total}\n"
        f"Antall oppmøtedager siste syv dager: {num_days_last_7}\n"
        f"Antall oppmøtedager siste tretti dager: {num_days_last_30}\n"
        f"Lengste dag: {longest_day_date}, {longest_day_duration:.0f} timer\n"
        f"Tidligste oppmøte: {earliest_arrival.strftime('%H:%M:%S')}\n"
        f"Seneste avreise: {latest_departure.strftime('%H:%M:%S')}\n"
    )
    return message


def main():
    with open(USER_JSON_FILE, "r") as f:
        usernames: dict[str, str] = json.load(f)

    user_input = "dummy"
    logged_in_user = None
    clear_and_print(GREETING)

    while user_input.lower() not in ["q", "quit"]:
        user_input = input(PROMPT)

        # Log card read
        if user_input.isnumeric() and len(user_input) == 10:
            logged_in_user = User(user_input, usernames)

            # Print hello message
            clear_and_print("Velkommen " + highlight(logged_in_user.username) + "!")
            if logged_in_user.username == logged_in_user.card_num:
                print("Du kan sette et brukernavn ved å trykke 'u' etterfulgt av 'enter'.")

            # Log entry and update stats, print streak
            now = dt.datetime.now(tz=dt.UTC)
            log_entry(logged_in_user, now)
            update_stats(logged_in_user, now)
            print(("🔥" * logged_in_user.stat_df.iloc[-1].loc["current_streak"]) + "\n")

        match user_input:
            # Log out / reset screen
            case "":
                logged_in_user = None
                clear_and_print(GREETING)

            # Show instructions
            case "i":
                clear_and_print(INSTRUCTIONS)

            # Show statistics
            case "s":
                clear_and_print(get_statistics_message(logged_in_user))

            # Change username
            case "u":
                if logged_in_user is None:
                    clear_and_print("Må tæppe kort først!\n")
                else:
                    new_username = input(f"Skriv inn brukernavn for bruker med kortnummer {highlight(logged_in_user.card_num)}: ")
                    update_username(usernames, logged_in_user.card_num, new_username)
                    clear_and_print(f"Brukernavn {highlight(new_username)} registrert for kort {highlight(logged_in_user.card_num)}")

            # Admin feature, rerun stat counts
            case "RERUN":
                for statsfile in os.listdir(STATS_DIR):
                    os.remove(STATS_DIR / statsfile)
                for logfile in sorted(os.listdir(LOG_DIR)):
                    for line in (LOG_DIR / logfile).read_text().split("\n")[:-1]:
                        timestamp, card_num = line.split(",")
                        update_stats(User(card_num, usernames), dt.datetime.fromisoformat(timestamp))

if __name__ == "__main__":
    main()
