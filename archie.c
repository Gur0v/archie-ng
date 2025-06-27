#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <stdbool.h>
#include <errno.h>
#include <sys/wait.h>

#define VERSION "2.0.0"
#define MAX_INPUT 512
#define MAX_COMMAND 1024
#define MAX_PATH 256

typedef enum {
    PACKAGE_MANAGER_NONE,
    PACKAGE_MANAGER_PARU,
    PACKAGE_MANAGER_YAY,
    PACKAGE_MANAGER_PACMAN
} PackageManager;

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
} Command;

typedef struct {
    PackageManager manager;
    const char *binary;
    const char *name;
} ManagerInfo;

static const ManagerInfo managers[] = {
    {PACKAGE_MANAGER_PARU, "paru", "paru"},
    {PACKAGE_MANAGER_YAY, "yay", "yay"},
    {PACKAGE_MANAGER_PACMAN, "pacman", "pacman"}
};

static const size_t manager_count = sizeof(managers) / sizeof(managers[0]);

static bool command_exists(const char *command) {
    char path[MAX_COMMAND];
    int ret = snprintf(path, sizeof(path), "command -v %s >/dev/null 2>&1", command);
    
    if (ret < 0 || ret >= (int)sizeof(path)) {
        return false;
    }
    
    return system(path) == 0;
}

static PackageManager detect_package_manager(void) {
    for (size_t i = 0; i < manager_count; i++) {
        if (command_exists(managers[i].binary)) {
            return managers[i].manager;
        }
    }
    return PACKAGE_MANAGER_NONE;
}

static const char *get_manager_binary(PackageManager manager) {
    for (size_t i = 0; i < manager_count; i++) {
        if (managers[i].manager == manager) {
            return managers[i].binary;
        }
    }
    return NULL;
}

static const char *get_manager_name(PackageManager manager) {
    for (size_t i = 0; i < manager_count; i++) {
        if (managers[i].manager == manager) {
            return managers[i].name;
        }
    }
    return "unknown";
}

static bool safe_input(char *buffer, size_t size, const char *prompt) {
    if (!buffer || size == 0) {
        return false;
    }
    
    printf("%s", prompt);
    fflush(stdout);
    
    if (!fgets(buffer, (int)size, stdin)) {
        return false;
    }
    
    size_t len = strlen(buffer);
    if (len > 0 && buffer[len - 1] == '\n') {
        buffer[len - 1] = '\0';
        len--;
    }
    
    return len > 0;
}

static bool validate_package_name(const char *package) {
    if (!package || strlen(package) == 0) {
        return false;
    }
    
    for (const char *p = package; *p; p++) {
        if (*p == ';' || *p == '&' || *p == '|' || *p == '`' || 
            *p == '$' || *p == '(' || *p == ')' || *p == '<' || *p == '>') {
            return false;
        }
    }
    
    return true;
}

static int execute_command(const char *command) {
    if (!command) {
        return -1;
    }
    
    int status = system(command);
    
    if (status == -1) {
        perror("Command execution failed");
        return -1;
    }
    
    return WEXITSTATUS(status);
}

static bool build_command(char *buffer, size_t size, const char *manager, 
                         const char *args, const char *package) {
    int ret;
    
    if (package && !validate_package_name(package)) {
        fprintf(stderr, "Error: Invalid package name\n");
        return false;
    }
    
    if (package) {
        ret = snprintf(buffer, size, "%s %s %s", manager, args, package);
    } else {
        ret = snprintf(buffer, size, "%s %s", manager, args);
    }
    
    if (ret < 0 || ret >= (int)size) {
        fprintf(stderr, "Error: Command too long\n");
        return false;
    }
    
    return true;
}

static void update_system(PackageManager manager) {
    const char *binary = get_manager_binary(manager);
    char command[MAX_COMMAND];
    
    printf("Updating system packages...\n");
    
    if (!build_command(command, sizeof(command), binary, "-Syu", NULL)) {
        return;
    }
    
    execute_command(command);
}

static void install_package(PackageManager manager) {
    const char *binary = get_manager_binary(manager);
    char package[MAX_INPUT];
    char command[MAX_COMMAND];
    
    if (!safe_input(package, sizeof(package), "Package to install: ")) {
        printf("Installation cancelled\n");
        return;
    }
    
    if (!build_command(command, sizeof(command), binary, "-S", package)) {
        return;
    }
    
    printf("Installing %s...\n", package);
    execute_command(command);
}

static void remove_package(PackageManager manager) {
    const char *binary = get_manager_binary(manager);
    char package[MAX_INPUT];
    char command[MAX_COMMAND];
    
    if (!safe_input(package, sizeof(package), "Package to remove: ")) {
        printf("Removal cancelled\n");
        return;
    }
    
    if (!build_command(command, sizeof(command), binary, "-R", package)) {
        return;
    }
    
    printf("Removing %s...\n", package);
    execute_command(command);
}

static void purge_package(PackageManager manager) {
    const char *binary = get_manager_binary(manager);
    char package[MAX_INPUT];
    char command[MAX_COMMAND];
    
    if (!safe_input(package, sizeof(package), "Package to purge: ")) {
        printf("Purge cancelled\n");
        return;
    }
    
    if (!build_command(command, sizeof(command), binary, "-Rns", package)) {
        return;
    }
    
    printf("Purging %s...\n", package);
    execute_command(command);
}

static void search_package(PackageManager manager) {
    const char *binary = get_manager_binary(manager);
    char query[MAX_INPUT];
    char command[MAX_COMMAND];
    
    if (!safe_input(query, sizeof(query), "Search query: ")) {
        printf("Search cancelled\n");
        return;
    }
    
    if (!build_command(command, sizeof(command), binary, "-Ss", query)) {
        return;
    }
    
    execute_command(command);
}

static void clean_cache(PackageManager manager) {
    const char *binary = get_manager_binary(manager);
    char command[MAX_COMMAND];
    
    printf("Cleaning package cache...\n");
    
    if (!build_command(command, sizeof(command), binary, "-Sc", NULL)) {
        return;
    }
    
    execute_command(command);
}

static void remove_orphans(PackageManager manager) {
    const char *binary = get_manager_binary(manager);
    char command[MAX_COMMAND];
    
    printf("Removing orphaned packages...\n");
    
    if (manager == PACKAGE_MANAGER_PACMAN) {
        if (!build_command(command, sizeof(command), binary, 
                          "-Rns $(pacman -Qtdq)", NULL)) {
            return;
        }
    } else {
        if (!build_command(command, sizeof(command), binary, "-c", NULL)) {
            return;
        }
    }
    
    execute_command(command);
}

static void display_help(void) {
    printf("\nArchie Package Manager Commands:\n");
    printf("  u - Update system packages\n");
    printf("  i - Install package\n");
    printf("  r - Remove package\n");
    printf("  p - Purge package (remove with dependencies)\n");
    printf("  s - Search packages\n");
    printf("  c - Clean package cache\n");
    printf("  o - Remove orphaned packages\n");
    printf("  h - Show this help\n");
    printf("  q - Quit\n\n");
}

static void display_version(void) {
    printf("Archie Package Manager v%s\n", VERSION);
    printf("A modern package management frontend for Arch Linux\n");
    printf("Supporting paru, yay, and pacman\n\n");
}

static Command parse_command(const char *input) {
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

static bool handle_command(Command cmd, PackageManager manager) {
    switch (cmd) {
        case CMD_UPDATE:
            update_system(manager);
            break;
        case CMD_INSTALL:
            install_package(manager);
            break;
        case CMD_REMOVE:
            remove_package(manager);
            break;
        case CMD_PURGE:
            purge_package(manager);
            break;
        case CMD_SEARCH:
            search_package(manager);
            break;
        case CMD_CLEAN:
            clean_cache(manager);
            break;
        case CMD_ORPHANS:
            remove_orphans(manager);
            break;
        case CMD_HELP:
            display_help();
            break;
        case CMD_QUIT:
            return false;
        case CMD_INVALID:
            printf("Invalid command. Type 'h' for help.\n");
            break;
    }
    return true;
}

static bool install_dependencies(void) {
    char response[MAX_INPUT];
    
    printf("Git is required but not installed.\n");
    if (!safe_input(response, sizeof(response), "Install git? (y/N): ")) {
        return false;
    }
    
    if (response[0] != 'y' && response[0] != 'Y') {
        return false;
    }
    
    printf("Installing git...\n");
    return execute_command("sudo pacman -S --needed git") == 0;
}

static bool install_aur_helper(void) {
    char response[MAX_INPUT];
    
    if (!safe_input(response, sizeof(response), "Install paru AUR helper? (y/N): ")) {
        return false;
    }
    
    if (response[0] != 'y' && response[0] != 'Y') {
        return false;
    }
    
    if (!command_exists("git") && !install_dependencies()) {
        fprintf(stderr, "Cannot install AUR helper without git\n");
        return false;
    }
    
    printf("Installing paru...\n");
    
    const char *install_script = 
        "cd /tmp && "
        "git clone https://aur.archlinux.org/paru.git && "
        "cd paru && "
        "makepkg -si --noconfirm && "
        "cd .. && "
        "rm -rf paru";
    
    return execute_command(install_script) == 0;
}

static int run_single_command(const char *cmd_str, PackageManager manager) {
    Command cmd = parse_command(cmd_str);
    
    if (cmd == CMD_INVALID) {
        fprintf(stderr, "Invalid command: %s\n", cmd_str);
        return 1;
    }
    
    if (cmd == CMD_QUIT) {
        return 0;
    }
    
    handle_command(cmd, manager);
    return 0;
}

static void run_interactive_mode(PackageManager manager) {
    char input[MAX_INPUT];
    
    printf("Archie Package Manager - Interactive Mode\n");
    printf("Using: %s\n", get_manager_name(manager));
    printf("Type 'h' for help, 'q' to quit\n\n");
    
    while (true) {
        if (!safe_input(input, sizeof(input), "archie> ")) {
            continue;
        }
        
        Command cmd = parse_command(input);
        if (!handle_command(cmd, manager)) {
            break;
        }
        
        printf("\n");
    }
    
    printf("Goodbye!\n");
}

int main(int argc, char *argv[]) {
    if (argc > 1) {
        if (strcmp(argv[1], "--version") == 0 || strcmp(argv[1], "-v") == 0) {
            display_version();
            return 0;
        }
        
        if (strcmp(argv[1], "--help") == 0 || strcmp(argv[1], "-h") == 0) {
            display_version();
            display_help();
            return 0;
        }
        
        if (argc == 3 && strcmp(argv[1], "--exec") == 0) {
            PackageManager manager = detect_package_manager();
            
            if (manager == PACKAGE_MANAGER_NONE) {
                fprintf(stderr, "No supported package manager found\n");
                if (!install_aur_helper()) {
                    return 1;
                }
                manager = detect_package_manager();
                if (manager == PACKAGE_MANAGER_NONE) {
                    fprintf(stderr, "Failed to install package manager\n");
                    return 1;
                }
            }
            
            return run_single_command(argv[2], manager);
        }
    }
    
    PackageManager manager = detect_package_manager();
    
    if (manager == PACKAGE_MANAGER_NONE) {
        printf("No supported package manager found.\n");
        printf("Supported managers: paru, yay, pacman\n\n");
        
        if (!install_aur_helper()) {
            printf("Cannot proceed without a package manager.\n");
            return 1;
        }
        
        manager = detect_package_manager();
        if (manager == PACKAGE_MANAGER_NONE) {
            fprintf(stderr, "Installation failed\n");
            return 1;
        }
        
        printf("Installation successful! Restarting...\n\n");
    }
    
    run_interactive_mode(manager);
    return 0;
}
