#import <Foundation/Foundation.h>
#include <signal.h>
#include <unistd.h>

static volatile sig_atomic_t g_keep_running = 1;

static void handle_signal(int) {
    g_keep_running = 0;
}

int main(void) {
    @autoreleasepool {
        signal(SIGTERM, handle_signal);
        signal(SIGINT, handle_signal);
        NSLog(@"macfriends-host started pid=%d", getpid());
        while (g_keep_running) {
            [NSThread sleepForTimeInterval:1.0];
        }
        NSLog(@"macfriends-host stopping");
    }
    return 0;
}
