# Backups of Denial

This program makes a timestamped copy of your Save file each time the game writes to it.
The goal is to deny hackers the chance to corrupt your save file.

This runs as a separate process from the game; it is not a mod. You won't be banned for using it.

## Setup

- Download from [Releases](https://github.com/usrbinsam/backups-of-denial/releases)
- Create the config file: `C:\Users\YOUR USERNAME\.backups-of-denial.toml`

```toml
backup_dir = '''\path\for\backup\storage'''
save_game_dir = '''C:\Users\YOUR USERNAME\OneDrive\Documents\NBGI\DARK SOULS REMASTERED'''
```

- Start the executable: `backups-of-denial.exe`
    - :exclamation: You will likely get a "Windows SmartScreen" alert for the first run.
      Click **More Info** then **Run anyway**.
- Start playing

