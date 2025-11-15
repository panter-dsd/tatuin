# Tatuin (Task Aggregator TUI for N providers)

Tatuin is a task aggregation tool that allows you to manage and track your tasks
from various providers in one convenient place.
Currently, the project supports the next providers:

| Provider name        | List | Create | Update | Delete | Status change    |
| -------------------- | ---- | ------ | ------ | ------ | ---------------- |
| Obsidian             | ✅   | ✅     | ✅     | ✅     | ✅               |
| Todoist              | ✅   | ✅     | ✅     | ✅     | ✅<sup>(1)</sup> |
| GitLab TODO          | ✅   | ❌     | ❌     | ❌     | ✅<sup>(1)</sup> |
| GitHub Issues        | ✅   | ❌     | ❌     | ❌     | ❌               |
| iCal<sup>(2)</sup>   | ✅   | ❌     | ❌     | ❌     | ❌               |
| CalDav<sup>(3)</sup> | ✅   | ✅     | ✅     | ✅     | ✅               |

(1): the provider supports the only Complete/Not complete statuses

(2): any provider that provides Calendar Subscription URL

(3): any provider that implements CalDav protocol (NextCloud for instance)

Tatuin provides users with an easy-to-use Text User Interface (TUI) for viewing and managing their tasks.

## Features

- **Cross-provider Task Management:** Tatuin allows you to create(Todoist and Obsidian), view and manage tasks across different task management platforms.
- **Command-line Interface (CLI):** The project is designed using a text-based interface, making it accessible from the command line.
- **Task Filters & Status Changes:** Easily filter and update your tasks' statuses as needed.
- **Save and load UI state:** The user can save the current view's state (selected provider, selected project, used filters) and switch between states.
- **Theming support**: The user can choose between themes or create their own.

## Quick Start

### Installation

1. Install via cargo

   ```bash
   cargo install tatuin
   ~/.cargo/bin/tatuin --help
   ```

2. Install via homebrew

   ```bash
   brew install panter-dsd/tap/tatuin
   ```

### Adding New Providers

To add a new provider, use the following command:

```bash
tatuin add-provider
```

This command will guide you through setting up the integration for the specified provider.

### Task Creation and Editing support

Currently, only the Todoist and Obsidian providers support task creation and editing. The Todoist provider is out of the box, but for Obsidian,
you must use the [obsidian-local-rest-api](https://github.com/coddingtonbear/obsidian-local-rest-api) plugin.
You can find it under 'Local REST API' in Obsidian's community plugins.
The full information about the installation and configuration process can be found within Obsidian's UI.
Tatuin works seamlessly with both secure and insecure configurations, but note that you must install a certificate for the secure setup (refer to the [wiki](https://github.com/coddingtonbear/obsidian-web/wiki/How-do-I-get-my-browser-trust-my-Obsidian-Local-REST-API-certificate%3F) for details).

#### Shortcuts (they work only when the tasks list panel is active)

- a: Create a task
- e: Edit the task under cursor

### Theming Support

Tatuin includes theming support, allowing you to customize the application's appearance to suit your preferences. To use a new theme, download a theme file (for instance, [nord.theme](https://github.com/panter-dsd/tatuin/blob/master/assets/themes/nord.theme)) and save it into the configuration directory: `tatuin config-dir`. For example, in Linux you might place a theme file as `~/.config/tatuin/nord.theme`.

You can activate a theme at launch by using the `--theme` command-line option followed by the theme name: `tatuin --theme nord`

Alternatively, to set a default theme, edit the configuration file with your favourite editor. For instance, if you like vim, use the command `vim "$(tatuin config-dir)/settings.toml"` and specify the theme name as follows:

```toml
theme = "nord"
```

This feature enables seamless switching between themes, offering both flexibility and a personalized experience.

## Screenshots

### Main window

![Main screenshot](https://raw.github.com/panter-dsd/tatuin/master/assets/screenshots/main.png?raw=true "Main screenshot")

### Create a task

![Task creation dialog](https://raw.github.com/panter-dsd/tatuin/master/assets/screenshots/task_creation_dialog.png?raw=true "Task creation dialog")

### Edit the task (Nord theme)

![Task editing dialog](https://raw.github.com/panter-dsd/tatuin/master/assets/screenshots/task_editing_nord_theme.png?raw=true "Task editing dialog")

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## Announces and Feedback

You can find announcements about new functionality on the Telegram [channel](https://t.me/tatuin_project).
I'd be glad to receive any feedback from you.

## License

Distributed under the MIT License. See `LICENSE.txt` for more information.

---

Tatuin is a growing project with plans to add many more providers in future releases. Stay tuned and join us on this journey of improving task management!

For any questions or feedback, please feel free to open an issue on GitHub!
