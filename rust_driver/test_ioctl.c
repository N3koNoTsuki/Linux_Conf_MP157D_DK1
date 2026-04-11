#include <stdio.h>
#include <stdint.h>
#include <fcntl.h>
#include <unistd.h>
#include <string.h>
#include <sys/ioctl.h>

#define DS3231_GET_SECONDS  _IOR('d', 0x01, uint8_t)
#define DS3231_GET_MINUTES  _IOR('d', 0x02, uint8_t)
#define DS3231_GET_HOURS    _IOR('d', 0x03, uint8_t)
#define DS3231_GET_PM       _IOR('d', 0x04, uint8_t)
#define DS3231_GET_DAYS     _IOR('d', 0x05, uint8_t)
#define DS3231_GET_DATE     _IOR('d', 0x06, uint8_t)
#define DS3231_GET_MONTH    _IOR('d', 0x07, uint8_t)
#define DS3231_GET_YEAR     _IOR('d', 0x08, uint8_t)
#define DS3231_GET_TEMP     _IOWR('d', 0x09, int16_t)

#define DEBUG 1


int main(void) {

    uint8_t seconds = 0;
    uint8_t minutes = 0;
    uint8_t hours = 0;
    uint8_t days = 0;
    uint8_t date = 0;
    uint8_t month = 0;
    uint16_t year = 0;
    int16_t temp = 0;

    int fd = open("/dev/ds3231", O_RDONLY);
    if (fd < 0) {
        perror("open");
        return 1;
    }

    if (ioctl(fd, DS3231_GET_SECONDS, &seconds) < 0) {
        perror("ioctl");
        close(fd);
        return 1;
    }

    if (ioctl(fd, DS3231_GET_MINUTES, &minutes) < 0) {
        perror("ioctl");
        close(fd);
        return 1;
    }

    if (ioctl(fd, DS3231_GET_HOURS, &hours) < 0) {
        perror("ioctl");
        close(fd);
        return 1;
    }


    if (ioctl(fd, DS3231_GET_DAYS, &days) < 0) {
        perror("ioctl");
        close(fd);
        return 1;
    }


    if (ioctl(fd, DS3231_GET_DATE, &date) < 0) {
        perror("ioctl");
        close(fd);
        return 1;
    }

    if (ioctl(fd, DS3231_GET_MONTH, &month) < 0) {
        perror("ioctl");
        close(fd);
        return 1;
    }

    if (ioctl(fd, DS3231_GET_YEAR, &year) < 0) {
        perror("ioctl");
        close(fd);
        return 1;
    }

    if (ioctl(fd, DS3231_GET_TEMP, &temp) < 0) {
        perror("ioctl");
        close(fd);
        return 1;
    }
   
    char day_str[10];
    switch (days) {
        case 1: strcpy(day_str, "Monday"); break;
        case 2: strcpy(day_str, "Tuesday"); break;
        case 3: strcpy(day_str, "Wednesday"); break;
        case 4: strcpy(day_str, "Thursday"); break;
        case 5: strcpy(day_str, "Friday"); break;
        case 6: strcpy(day_str, "Saturday"); break;
        case 7: strcpy(day_str, "Sunday"); break;
        default: strcpy(day_str, "Unknown"); break;
    }
    
    char month_str[10];
    switch (month) {
        case 1: strcpy(month_str, "January"); break;
        case 2: strcpy(month_str, "February"); break;
        case 3: strcpy(month_str, "March"); break;
        case 4: strcpy(month_str, "April"); break;
        case 5: strcpy(month_str, "May"); break;
        case 6: strcpy(month_str, "June"); break;
        case 7: strcpy(month_str, "July"); break;
        case 8: strcpy(month_str, "August"); break;
        case 9: strcpy(month_str, "September"); break;
        case 10: strcpy(month_str, "October"); break;
        case 11: strcpy(month_str, "November"); break;
        case 12: strcpy(month_str, "December"); break;
        default: strcpy(month_str, "Unknown"); break;
    }

    printf("%s %u %s %u %u:%u:%u %.2f C\n", day_str, date, month_str, year, hours, minutes, seconds, temp / 4.0f);

    close(fd);
    return 0;
}