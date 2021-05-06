# dsa-cli

A simple helper tool for the pen & paper game "Das Schwarze Auge.
You can load a character sheet created with "The Dark Aid" (the file should have the extension `.tdc`) and perform various kinds of checks using their stats.

## Usage

The simplest way to use this, is to call it from the commandline.
Run "dsa-cli help" for a list of available options.

Additionally, you can create a discord bot by running "dsa-cli discord". This requires that you have created a discord bot account and added a login token to the config file ("discord -> login_token").

To create a Bot visit [https://discord.com/developers](https://discord.com/developers), login and press `New Application`.  
Enter a name and search for the tab `Bot`.  
Here you need to press `Add Bot` and configure it as you wish.
The checkbox `SERVER MEMBERS INTENT` needs to be checked.  

Once your bot is up and running, you can invite it to your server using the URL [https://discord.com/oauth2/authorize?scope=bot&client_id=<application_id>](https://discord.com/oauth2/authorize?scope=bot&client_id=<application_id>) where `<application_id>` is replaced by the value found in `General Information`.  
Instead of inviting it you can also message it directly.

The bot will now try to interpret any message sent to him (in server channels or private messages) starting with `!` as a command. Write `!help` for a list of commands.

The server permission `Manage Nicknames` and the channel permissions `View Channel`, `Send Messages` are required.  
It is also favorable to have the Bot posses a role listed above all other roles with the `Manage Nicknames` permission. This ensures it being able to rename all users except the server owner.

## Configuration

When first run, a config folder and default config files (`config.json`, `dsa_data.json`) will be created. The location depends on your operating system:

* Linux: `$HOME/.config/dsa-cli/`
* Windows: `%appdata%/dsa-cli/`
* MacOS: `$HOME/Library/Application Support/dsa-cli/`

You can change the config folder location by setting the environment variable `DSA_CLI_CONFIG_DIR`, though this shouldn't be necessary usually.

Both files can be edited to customize the behavior and rules. Here is a list of the options in `config.json`:

* `auto_update_dsa_data`\
    **Type:** Boolean\
    **Default:** true

    If true, the dsa_data.json file (which contains information on talents, etc.) will automatically be updated when starting a new version of dsa-cli. Note that this will erase any changes you make to that file.
* `dsa_rules`
    * `crit_rules`\
        **Type:** String\
        **Default:** DefaultCrits

        The rules for critial successes and failures in talent and spell checks:
        * NoCrits: No crits will be rolled
        * DefaultCrits: The official crit rules: Two 1s (20s) constitute a critical success (failure)
        * AlternativeCrits: A single 1 (20) constitutes a critical success (failure) which has to be confirmed by a second roll
* `discord`
    * `login_token`\
        **Type:** String

        The login token for a discord bot account. Required if you want to run a bot.
    * `require_complete_command`\
        **Type:** Boolean\
        **Default:** false

        If true, the bot reacts to commands of the form `!dsa-cli [SUB_COMMAND]`. 
        Otherwise, you can use `![SUB_COMMAND]`
    
    * `use_reply`\
        **Type:** Boolean\
        **Default:** false

        If true, the bot will send its response as a reply to the message that triggered it.
    * `max_attachement_size`\
        **Type:** Integer\
        **Default:** 1,000,000

        The maximum attachement size that the bot downloads. This currently applies just to the `!upload` command.
    * `max_name_length`\
        **Type:** Integer\
        **Default:** 32

        The maximum character name length. `.tdc` files that contain a longer name will be rejected.
  


## Docker

If you want to use the discord bot, you can also run it as a docker container.

The Dockerfile automatically sets the config folder to `/dsa-cli-config`,  
it is recommended to create a mount or volume for this folder in order to preserve configuration (such as the required discord token) and uploaded characters.

Also note that the first build will take some time, as all the dependencies have to be compiled.
Subsequent builds should be faster due to the docker cache.
