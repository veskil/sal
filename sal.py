from pathlib import Path
import datetime as dt
import os
import colorama
import json

ROOT = Path(__file__).parent
LOG_DIR = ROOT / "logs/"
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


def update_username(users: dict[str, str], user_id: str, new_username: str):
    users[user_id] = new_username
    with open(USER_JSON_FILE, "w") as f:
        json.dump(users, f, indent=0)


def clear_and_print(message):
    os.system("clear")
    print(message)


def log_entry(card_num: str) -> None:
    now = dt.datetime.now(tz=dt.UTC)
    log_message = f"{now.isoformat()},{card_num}\n"
    log_file = LOG_DIR / (now.strftime("%Y%m%d") + ".log")
    with open(log_file, "a") as file:
        file.write(log_message)


def main():
    with open(USER_JSON_FILE, "r") as f:
        users: dict[str, str] = json.load(f)
    user_input = "dummy"
    logged_in_user_id = None
    clear_and_print(GREETING)

    while user_input.lower() not in ["q", "quit"]:
        user_input = input(PROMPT)

        # Log card read
        if user_input.isnumeric() and len(user_input) == 10:
            logged_in_user_id = user_input
            log_entry(logged_in_user_id)
            if logged_in_user_id in users:
                clear_and_print("Velkommen " + highlight(users[logged_in_user_id]) + "!\n")
            else:
                update_username(users, logged_in_user_id, logged_in_user_id)
                clear_and_print("Kortnummer " + highlight(user_input)
                                + " registrert! Gjerne sett et brukernavn!\n")


        match user_input:
            # Log out / reset screen
            case "":
                logged_in_user_id = None
                clear_and_print(GREETING)

            # Show instructions
            case "i":
                clear_and_print(INSTRUCTIONS)

            # Show statistics
            case "s":
                clear_and_print("Brukerstatistikk er ikke implementert ennå :(\n")

            # Change username
            case "u":
                if logged_in_user_id is None:
                    clear_and_print("Må tæppe kort først!\n")
                else:
                    new_username = input(f"Skriv inn brukernavn for kort med nummer {highlight(logged_in_user_id)}: ")
                    update_username(users, logged_in_user_id, new_username)
                    clear_and_print(f"Brukernavn {highlight(new_username)} registrert for kort {highlight(logged_in_user_id)}")


if __name__ == "__main__":
    main()
