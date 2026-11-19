#include <dlfcn.h>

int main(void)
{
    // Match the names tried by winit's xkbcommon-dl. Resolve dependencies too.
    void *adapter = dlopen("libxkbcommon-x11.so.0", RTLD_NOW);
    if (!adapter)
        adapter = dlopen("libxkbcommon-x11.so", RTLD_NOW);
    return adapter ? 0 : 1;
}
