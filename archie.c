#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <stdbool.h>
#include <sys/wait.h>
#include <readline/readline.h>
#include <readline/history.h>

#define VERSION "2.1.0"
#define MAX_INPUT 512
#define MAX_COMMAND 2048

// Package manager types
typedef enum { 
    PKG_NONE, 
    PKG_PARU, 
    PKG_YAY, 
    PKG_PACMAN 
} pkg_manager_t;

// Command types
typedef enum { 
    CMD_UPDATE, 
    CMD_INSTALL, 
    CMD_REMOVE, 
    CMD_PURGE, 
    CMD_SEARCH, 
    CMD_CLEAN, 
    CMD_ORPHANS, 
    CMD_HELP, 
    CMD_QUIT, 
    CMD_INVALID 
} command_t;

// Package manager definitions
static const struct {
    pkg_manager_t type;
    const char *binary;
    const char *name;
} package_managers[] = {
    {PKG_PARU, "paru", "paru"},
    {PKG_YAY, "yay", "yay"},
    {PKG_PACMAN, "pacman", "pacman"}
};

static pkg_manager_t current_manager;
static char **completions = NULL;
static int completion_count = 0;

// ========== UTILITY FUNCTIONS ==========

// Check if command exists in PATH
static bool command_exists(const char *cmd) {
    if (!cmd || strlen(cmd) == 0) return false;
    
    char test_cmd[MAX_COMMAND];
    int ret = snprintf(test_cmd, sizeof(test_cmd), "command -v %s >/dev/null 2>&1", cmd);
    
    if (ret < 0 || ret >= (int)sizeof(test_cmd)) return false;
    return system(test_cmd) == 0;
}

// Detect available package manager
static pkg_manager_t detect_package_manager(void) {
    for (int i = 0; i < 3; i++) {
        if (command_exists(package_managers[i].binary)) {
            return package_managers[i].type;
        }
    }
    return PKG_NONE;
}

// Get binary name for package manager
static const char *get_binary_name(pkg_manager_t manager) {
    for (int i = 0; i < 3; i++) {
        if (package_managers[i].type == manager) {
            return package_managers[i].binary;
        }
    }
    return NULL;
}

// Get display name for package manager
static const char *get_display_name(pkg_manager_t manager) {
    for (int i = 0; i < 3; i++) {
        if (package_managers[i].type == manager) {
            return package_managers[i].name;
        }
    }
    return "unknown";
}

// Validate package name - prevent command injection
static bool is_valid_package_name(const char *name) {
    if (!name || strlen(name) == 0 || strlen(name) > MAX_INPUT) {
        return false;
    }
    
    // Check for dangerous characters
    const char *dangerous = ";;&|`$()<>\"'\\";
    for (const char *c = name; *c; c++) {
        if (strchr(dangerous, *c)) {
            return false;
        }
    }
    
    // Must start with alphanumeric or allowed special chars
    if (!(*name >= 'a' && *name <= 'z') && 
        !(*name >= 'A' && *name <= 'Z') && 
        !(*name >= '0' && *name <= '9') &&
        *name != '-' && *name != '_' && *name != '+') {
        return false;
    }
    
    return true;
}

// Execute command safely
static int execute_command(const char *cmd) {
    if (!cmd) return -1;
    
    int status = system(cmd);
    if (status == -1) return -1;
    
    return WEXITSTATUS(status);
}

// ========== COMPLETION SYSTEM ==========

// Free completion cache
static void free_completions(void) {
    if (!completions) return;
    
    for (int i = 0; i < completion_count; i++) {
        free(completions[i]);
    }
    free(completions);
    completions = NULL;
    completion_count = 0;
}

// Package completion generator
static char *completion_generator(const char *text, int state) {
    static FILE *pipe = NULL;
    static char *line = NULL;
    static size_t line_size = 0;
    char command[MAX_COMMAND];
    int text_len = strlen(text);

    // First call - initialize
    if (state == 0) {
        free_completions();
        if (pipe) pclose(pipe);
        
        int ret = snprintf(command, sizeof(command), "%s -Slq | grep '^%s'", 
                          get_binary_name(current_manager), text);
        if (ret < 0 || ret >= (int)sizeof(command)) return NULL;
        
        pipe = popen(command, "r");
        if (!pipe) return NULL;
    }

    // Read matching packages
    while (getline(&line, &line_size, pipe) != -1) {
        // Remove newline
        line[strcspn(line, "\n")] = '\0';
        
        if (strncmp(line, text, text_len) == 0) {
            char *pkg = strdup(line);
            if (!pkg) continue;
            
            // Add to cache for cleanup
            completions = realloc(completions, sizeof(char*) * (completion_count + 1));
            if (completions) {
                completions[completion_count++] = strdup(pkg);
            }
            
            return pkg;
        }
    }

    // End of completion
    if (pipe) {
        pclose(pipe);
        pipe = NULL;
    }
    return NULL;
}

// Completion function for readline
static char **package_completion(const char *text, int start, int end) {
    (void)end;
    
    // Only complete if at beginning or after space
    if (start == 0 || rl_line_buffer[start - 1] == ' ') {
        return rl_completion_matches(text, completion_generator);
    }
    return NULL;
}

// ========== READLINE SETUP ==========

static void init_readline(void) {
    rl_readline_name = "archie-ng";
    rl_attempted_completion_function = package_completion;
    read_history(".archie-ng_history");
}

static void cleanup_readline(void) {
    write_history(".archie-ng_history");
    history_truncate_file(".archie-ng_history", 100);
    free_completions();
}

// Get user input safely
static char *get_input(const char *prompt) {
    char *input = readline(prompt);
    if (input && *input) {
        add_history(input);
    }
    return input;
}

// ========== PACKAGE OPERATIONS ==========

// Execute package manager command
static void execute_package_command(const char *args, const char *package) {
    char command[MAX_COMMAND];
    const char *binary = get_binary_name(current_manager);
    
    if (!binary) {
        printf("Error: Package manager not available\n");
        return;
    }
    
    if (package) {
        if (!is_valid_package_name(package)) {
            printf("Error: Invalid package name\n");
            return;
        }
        snprintf(command, sizeof(command), "%s %s %s", binary, args, package);
    } else {
        snprintf(command, sizeof(command), "%s %s", binary, args);
    }
    
    execute_command(command);
}

// Update system
static void update_system(void) {
    printf("Updating system...\n");
    execute_package_command("-Syu", NULL);
}

// Install package
static void install_package(void) {
    char *package = get_input("Package to install: ");
    if (!package || strlen(package) == 0) {
        free(package);
        return;
    }
    
    printf("Installing %s...\n", package);
    execute_package_command("-S", package);
    free(package);
}

// Remove package
static void remove_package(void) {
    char *package = get_input("Package to remove: ");
    if (!package || strlen(package) == 0) {
        free(package);
        return;
    }
    
    printf("Removing %s...\n", package);
    execute_package_command("-R", package);
    free(package);
}

// Purge package (remove with dependencies)
static void purge_package(void) {
    char *package = get_input("Package to purge: ");
    if (!package || strlen(package) == 0) {
        free(package);
        return;
    }
    
    printf("Purging %s...\n", package);
    execute_package_command("-Rns", package);
    free(package);
}

// Search packages
static void search_packages(void) {
    char *query = get_input("Search query: ");
    if (!query || strlen(query) == 0) {
        free(query);
        return;
    }
    
    execute_package_command("-Ss", query);
    free(query);
}

// Clean package cache
static void clean_cache(void) {
    printf("Cleaning package cache...\n");
    execute_package_command("-Sc", NULL);
}

// Remove orphaned packages
static void remove_orphans(void) {
    printf("Removing orphaned packages...\n");
    
    if (current_manager == PKG_PACMAN) {
        execute_command("pacman -Rns $(pacman -Qtdq) 2>/dev/null || echo 'No orphans found'");
    } else {
        execute_command("pacman -Qtdq | xargs -r pacman -Rns 2>/dev/null || echo 'No orphans found'");
    }
}

// Show help
static void show_help(void) {
    printf("\nCommands:\n");
    printf("  u - Update system      i - Install package\n");
    printf("  r - Remove package     p - Purge package\n");
    printf("  s - Search packages    c - Clean cache\n");
    printf("  o - Remove orphans     h - Show help\n");
    printf("  q - Quit\n");
    printf("\nTip: Use TAB for package name completion\n\n");
}

// ========== COMMAND PARSING ==========

// Parse single character command
static command_t parse_command(const char *input) {
    if (!input || strlen(input) != 1) {
        return CMD_INVALID;
    }
    
    switch (input[0]) {
        case 'u': return CMD_UPDATE;
        case 'i': return CMD_INSTALL;
        case 'r': return CMD_REMOVE;
        case 'p': return CMD_PURGE;
        case 's': return CMD_SEARCH;
        case 'c': return CMD_CLEAN;
        case 'o': return CMD_ORPHANS;
        case 'h': return CMD_HELP;
        case 'q': return CMD_QUIT;
        default:  return CMD_INVALID;
    }
}

// Handle command execution
static bool handle_command(command_t cmd) {
    switch (cmd) {
        case CMD_UPDATE:  update_system(); break;
        case CMD_INSTALL: install_package(); break;
        case CMD_REMOVE:  remove_package(); break;
        case CMD_PURGE:   purge_package(); break;
        case CMD_SEARCH:  search_packages(); break;
        case CMD_CLEAN:   clean_cache(); break;
        case CMD_ORPHANS: remove_orphans(); break;
        case CMD_HELP:    show_help(); break;
        case CMD_QUIT:    return false;
        case CMD_INVALID: 
            printf("Invalid command. Type 'h' for help.\n"); 
            break;
    }
    return true;
}

// ========== PARU INSTALLATION ==========

// Install paru AUR helper if needed
static bool install_paru(void) {
    char *response = get_input("Install paru AUR helper? (y/N): ");
    if (!response || (response[0] != 'y' && response[0] != 'Y')) {
        free(response);
        return false;
    }
    free(response);

    // Ensure git is installed
    if (!command_exists("git")) {
        printf("Installing git...\n");
        if (execute_command("sudo pacman -S --needed git") != 0) {
            return false;
        }
    }

    printf("Installing paru from AUR...\n");
    const char *install_cmd = 
        "cd /tmp && "
        "git clone https://aur.archlinux.org/paru.git && "
        "cd paru && "
        "makepkg -si --noconfirm && "
        "cd .. && "
        "rm -rf paru";
    
    return execute_command(install_cmd) == 0;
}

// ========== MAIN PROGRAM ==========

// Interactive mode
static void interactive_mode(void) {
    char *input;

    init_readline();

    printf("\nWelcome to Archie-ng v%s\n", VERSION);
    printf("Using %s package manager\n", get_display_name(current_manager));
    printf("Type 'h' for help\n\n");

    while ((input = get_input("$ ")) != NULL) {
        if (strlen(input) == 0) {
            free(input);
            continue;
        }
        
        command_t cmd = parse_command(input);
        bool should_continue = handle_command(cmd);
        free(input);
        
        if (!should_continue) break;
    }

    cleanup_readline();
    printf("\nGoodbye!\n");
}

int main(int argc, char *argv[]) {
    // Handle command line arguments
    if (argc > 1) {
        if (strcmp(argv[1], "--version") == 0 || strcmp(argv[1], "-v") == 0) {
            printf("Archie-ng v%s\n", VERSION);
            return 0;
        }
        if (strcmp(argv[1], "--help") == 0 || strcmp(argv[1], "-h") == 0) {
            printf("Archie-ng v%s - Arch Linux package manager wrapper\n", VERSION);
            show_help();
            return 0;
        }
    }

    // Detect package manager
    current_manager = detect_package_manager();

    if (current_manager == PKG_NONE) {
        printf("No supported package manager found (paru, yay, pacman)\n");
        
        if (!install_paru()) {
            printf("Cannot proceed without a package manager.\n");
            return 1;
        }
        
        // Re-detect after installation
        current_manager = detect_package_manager();
        if (current_manager == PKG_NONE) {
            printf("Paru installation failed.\n");
            return 1;
        }
    }

    interactive_mode();
    return 0;
}
