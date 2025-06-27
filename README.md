# Backups of Denial

This program makes a timestamped copy of your Save file each time the game writes to it.
If a hacker invader manages to corrupt your game save, you have the opportunity to roll back to a good save fle.

This runs as a separate process from the game; it is not a mod and therefore does not violate ToS.

## Setup

- Download from [Releases](https://github.com/usrbinsam/backups-of-denial/releases)
- Create the config file: `C:\Users\YOUR USERNAME\.backups-of-denial.toml`

```toml
backup_dir = '''\path\for\backup\storage'''
save_game_dir = '''C:\Users\YOUR USERNAME\OneDrive\Documents\NBGI\DARK SOULS REMASTERED'''
```

- Start the executable: `backups-of-denial.exe`
    - :exclamation: You will likely get a "Windows SmartScreen" alert for the first run.
      Click **More Info** > **Run anyway**.
- Start playing

## TODO

- Backup retention: oldest backups are not auto-removed.
- Game Launcher: Wrapper around the Game executable so the watcher can automatically start and stop with the game.