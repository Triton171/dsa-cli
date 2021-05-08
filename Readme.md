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

Next up it is time to give your BOT the required permissions and invite it to your server.  
To do this open the tab `OAuth2` and add the URI `http://localhost` as a redirect under `Redirects`.  
Now you are set to invite your bot.  
To do so open the URL [https://discord.com/api/oauth2/authorize?permissions=134217728&redirect_uri=http%3A%2F%2Flocalhost&scope=bot%20applications.commands&client_id=<client_id>](https://discord.com/api/oauth2/authorize?permissions=134217728&redirect_uri=http%3A%2F%2Flocalhost&scope=bot%20applications.commands&client_id=<client_id>) where `<client_id>` is replaced by the value found under `OAuth2 -> Client Information`.
Instead of inviting it you can also message it directly.

The bot will now try to interpret any message sent to him (in server channels or private messages) starting with `!` as a command. Write `!help` for a list of commands.

The server permission `Manage Nicknames` and the channel permissions `View Channel`, `Send Messages`, `Read Message History` are required.  
Uppon inviting the Bot to a Server a role is created with the same name your BOT posseses:
It is favorable to have either this role or a custom role with the `Manage Nicknames` permission and your Bot as a member listed above all other roles. This ensures it being able to rename all users except the server owner.

## Configuration

When first run, a config folder and default config files (`config.json`, `dsa_data.json`) will be created. The location depends on your operating system:

* Linux: `$HOME/.config/dsa-cli/`
* Windows: `%appdata%/dsa-cli/`
* MacOS: `$HOME/Library/Application Support/dsa-cli/`

You can change the config folder location by setting the environment variable `DSA_CLI_CONFIG_DIR`, though this shouldn't be necessary usually.

Both files can be edited to customize the behavior and rules.

## Docker

If you want to use the discord bot, you can also run it as a docker container.

The Dockerfile automatically sets the config folder to `/dsa-cli-config`,  
it is recommended to create a mount or volume for this folder in order to preserve configuration (such as the required discord token) and uploaded characters.

Also note that the first build will take some time, as all the dependencies have to be compiled.
Subsequent builds should be faster due to the docker cache.
