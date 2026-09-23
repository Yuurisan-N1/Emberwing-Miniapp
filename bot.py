import sys
import os
import signal
import requests
from colorama import Fore, Style, init
from utils.banner import show_banner

init(autoreset=False)

MY_PROJECT = "Emberwing Miniapp"

RELEASES = [
    {
        "name": "Linux x86_64",
        "filename": "emberwing-bot-linux-x86_64",
        "url": "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-linux-x86_64",
        "chmod": True,
    },
    {
        "name": "Linux aarch64",
        "filename": "emberwing-bot-linux-aarch64",
        "url": "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-linux-aarch64",
        "chmod": True,
    },
    {
        "name": "Linux armv7",
        "filename": "emberwing-bot-linux-armv7",
        "url": "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-linux-armv7",
        "chmod": True,
    },
    {
        "name": "Android aarch64",
        "filename": "emberwing-bot-android-aarch64",
        "url": "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-android-aarch64",
        "chmod": True,
    },
    {
        "name": "Android armv7",
        "filename": "emberwing-bot-android-armv7",
        "url": "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-android-armv7",
        "chmod": True,
    },
    {
        "name": "macOS aarch64",
        "filename": "emberwing-bot-macos-aarch64",
        "url": "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-macos-aarch64",
        "chmod": True,
    },
    {
        "name": "Windows x86_64",
        "filename": "emberwing-bot-windows-x86_64.exe",
        "url": "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-windows-x86_64.exe",
        "chmod": False,
    },
]

G = Style.BRIGHT + Fore.GREEN
Y = Style.BRIGHT + Fore.YELLOW
R = Style.BRIGHT + Fore.RED
RST = Style.RESET_ALL


def green(text):
    return f"{G}{text}{RST}"


def yellow(text):
    return f"{Y}{text}{RST}"


def red(text):
    return f"{R}{text}{RST}"


def signal_handler(sig, frame):
    sys.stdout.write("\n")
    sys.stdout.write(R + "Script stopped by user" + RST + "\n")
    sys.stdout.flush()
    sys.exit(0)


signal.signal(signal.SIGINT, signal_handler)


def download_file(url, filename, chmod):
    resp = requests.get(url, stream=True, timeout=60)
    total = int(resp.headers.get("content-length", 0))
    downloaded = 0
    chunk_size = 8192

    with open(filename, "wb") as f:
        for chunk in resp.iter_content(chunk_size=chunk_size):
            if chunk:
                f.write(chunk)
                downloaded += len(chunk)
                if total > 0:
                    percent = downloaded / total * 100
                    filled = int(percent / 2)
                    bar = " " * filled + "." * (50 - filled)
                    mb_done = downloaded / 1024 / 1024
                    mb_total = total / 1024 / 1024
                    line = f"\rDownloading {bar} {percent:.1f}% {mb_done:.2f} MB of {mb_total:.2f} MB   "
                    sys.stdout.write(G + line + RST)
                    sys.stdout.flush()
                else:
                    mb_done = downloaded / 1024 / 1024
                    line = f"\rDownloading {mb_done:.2f} MB   "
                    sys.stdout.write(Y + line + RST)
                    sys.stdout.flush()

    sys.stdout.write("\r" + " " * 80 + "\r")
    sys.stdout.flush()

    if chmod:
        os.chmod(filename, 0o755)


def main():
    show_banner(MY_PROJECT)

    print(yellow("Select the binary you want to download:"))
    print()
    for i, release in enumerate(RELEASES, start=1):
        print(green(f"{i}. {release['name']}"))
    print()

    while True:
        try:
            raw = input(Y + "Enter number: " + RST).strip()
            choice = int(raw)
            if 1 <= choice <= len(RELEASES):
                break
            print(red(f"Please enter a number between 1 and {len(RELEASES)}"))
        except ValueError:
            print(red("Invalid input please enter a valid number"))

    selected = RELEASES[choice - 1]
    print()
    print(yellow(f"Starting download {selected['name']}"))

    try:
        download_file(selected["url"], selected["filename"], selected["chmod"])
        print(green(f"Download complete saved as {selected['filename']}"))
        if selected["chmod"]:
            print(green("File permission set to executable"))
    except Exception:
        print(red("Download failed"))
        sys.exit(1)


if __name__ == "__main__":
    main()