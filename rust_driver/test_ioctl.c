#include <stdio.h>
#include <stdint.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/ioctl.h>

#define DS3231_IOCTL_GET_SECONDS _IOR('d', 0x01, uint8_t)

int main(void) {
    int fd = open("/dev/ds3231", O_RDONLY);
    if (fd < 0) {
        perror("open");
        return 1;
    }

    uint8_t seconds = 0;
    if (ioctl(fd, DS3231_IOCTL_GET_SECONDS, &seconds) < 0) {
        perror("ioctl");
        close(fd);
        return 1;
    }

    printf("seconds = %u\n", seconds);

    close(fd);
    return 0;
}
