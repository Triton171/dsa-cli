# dsa-cli

A simple helper tool for the pen & paper game "Das Schwarze Auge.
You can load a character sheet created with "The Dark Aid" (the file should have the extension `.tdc`) and perform various kinds of checks using their stats.

## Usage

The simplest way to use this, is to call it from the commandline.
Run "dsa-cli help" for a list of available options.

Additionally, you can use it as a discord bot. For information on how to set up a discord bot account, see below. If someone already hosts the bot, just use the link they give you to add the bot to your server. If you want to use the automatic rename function, you should move the bot role to the top of the role hierarchy, this ensures that the bot can rename everyone (unfortunately, the server owner can never be renamed).

To use the bot, simply send a command (prefaced by `!`) either to the bot directly, or in a text channel that the bot can access. Send `!help` for a list of available commands.

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

    * `num_threads`\
        **Type:** Integer\
        **Default:** 1

        The number of threads to use when running the discord bot. Use this, if you expect the bot to be heavily used by many people.
    * `require_complete_command`\
        **Type:** Boolean\
        **Default:** false

        If true, the bot reacts to commands of the form `!dsa-cli [SUB_COMMAND]`. 
        Otherwise, you can use `![SUB_COMMAND]`
    * `use_reply`\
        **Type:** Boolean\
        **Default:** true

        If true, the bot will send its response as a reply to the message that triggered it.
    * `max_attachement_size`\
        **Type:** Integer\
        **Default:** 1,000,000

        The maximum attachement size that the bot downloads. This currently applies just to the `!upload` command.
    * `max_name_length`\
        **Type:** Integer\
        **Default:** 32

        The maximum character name length. `.tdc` files that contain a longer name will be rejected.
  

## Hosting a discord bot
### Creating a bot account
Visit [https://discord.com/developers](https://discord.com/developers) and press `New Application`.
Enter a name and search for the tab `Bot`. Here you need to press `Add Bot` and configure it how you like. You will need to enable the `Server Members Intent` and, if you want other people to be able to use your bot, the `Public Bot` checkbox.

You can already copy the `Token`, you will need to enter this in the `login_token` field of the `config.json`.

Then, you have to go to the tab `OAuth2` and add the URL `http://localhost` under `Redirects`.
Now you can select this URL directly below and finally select the required scopes and permissions:\
Under `Scopes`, select:
* `bot`
* `applications.commands`

When doing this, a new section `Bot Permissions` should appear below. Here you have to select:
* `Manage Nicknames`
* `Send Messages`

Once you're done, you can copy the URL that discord generated for you. Anyone can use this to add your bot to their server.

The last step is to copy the `Application ID` from the `General Information` tab and add it, as well as your bot token, to the `config.json`.
### Using the docker container

If you want to use the discord bot, you can also run it as a docker container.

The Dockerfile automatically sets the config folder to `/dsa-cli-config`,  
it is recommended to create a mount or volume for this folder in order to preserve configuration (such as the required discord token) and uploaded characters.

Also note that the first build will take some time, as all the dependencies have to be compiled.
Subsequent builds should be faster due to the docker cache.
