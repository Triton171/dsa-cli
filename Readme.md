# dsa-cli

A simple helper tool for the pen & paper game "Das Schwarze Auge.
You can load a character sheet created with "The Dark Aid" (the file should have the extension `.tdc`) and perform various kinds of checks using their stats.

## Usage
The simplest way to use this, is to call it from the commandline.
Run "dsa-cli help" for a list of available options.

Additionally, you can create a discord bot by running "dsa-cli discord". This requires that you have created a discord bot account and added a login token to the config file ("discord -> login_token").

Once your bot is running, you can invite it to your server or message it directly. It will try to interpret any message starting with "!" as a command. Write "!help" for a list of commands.

## Configuration
When first run, a config folder and a default config file ("config.json") will be created. The location depends on your operating system:

* Linux: `$HOME/.config/dsa-cli/`
* Windows: `%appdata%/dsa-cli/`
* MacOS: `$HOME/Library/Application Support/dsa-cli/`

You can change the config folder location by setting the environment variable `DSA_CLI_CONFIG_DIR`, though this shouldn't be necessary usually.

The config file contains among other things:

* The currently loaded character
* Some customization options (such as the rules for critical successes/failures)
* A list of skills and associated attributes (this can be extended to include custom skills, spells, etc.)
* The login token for the discord bot account (if configured)

If you update the application, by default your old configuration file will not be replaced by a newer version (unless it is found incompatible). 
If you need the new version of the configuration file (for example if new spells have been added to the default config), you can manually rename or delete the old version to force an update.

## Docker
If you want to use the discord bot, you can also run it as a docker container. 

The Dockerfile automatically sets the config folder to `/dsa-cli-config`, 
it is recommended to create a mount or volume for this folder in order to preserve configuration (such as the required discord token) and uploaded characters.

Also note that the first build will take some time, as all the dependencies have to be compiled.
Subsequent builds should be faster due to the docker cache.