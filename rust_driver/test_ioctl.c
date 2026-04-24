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
#define DS3231_GET_YEAR     _IOR('d', 0x08, uint16_t)
#define DS3231_GET_TEMP     _IOWR('d', 0x09, int16_t)

// SET commands — user → kernel
#define DS3231_SET_SECONDS  _IOW('d', 0x0A, uint8_t)
#define DS3231_SET_MINUTES  _IOW('d', 0x0B, uint8_t)
#define DS3231_SET_HOURS    _IOWR('d', 0x0C, uint8_t)
#define DS3231_SET_12H      _IOWR('d', 0x0D, uint8_t)
#define DS3231_SET_PM       _IOWR('d', 0x0E, uint8_t)
#define DS3231_SET_DAYS     _IOW('d', 0x0F, uint8_t)
#define DS3231_SET_DATE     _IOW('d', 0x10, uint8_t)
#define DS3231_SET_MONTH    _IOWR('d', 0x11, uint8_t)
#define DS3231_SET_YEAR     _IOWR('d', 0x12, uint16_t)


int main(void) {

        int fd = open("/dev/ds3231", O_RDWR);
    if (fd < 0) {
        perror("open");
        return 1;
    }

     /* ------------------------------------------------------------------ */
    /* SET — écriture d'une date/heure connue dans le RTC                 */
    /* ------------------------------------------------------------------ */
    printf("\n--- SET ---\n");

    uint8_t  set_seconds = 0;
    uint8_t  set_minutes = 0;
    uint8_t  set_12h     = 0;
    uint8_t  set_pm      = 0;
    uint8_t  set_hours   = 0;
    uint8_t  set_days    = 0;
    uint8_t  set_date    = 0;
    uint8_t  set_month   = 0;
    uint16_t set_year    = 0;

    printf("Seconds (0-59)      : "); fflush(stdout); scanf("%hhu", &set_seconds);
    if (ioctl(fd, DS3231_SET_SECONDS, &set_seconds) < 0) { perror("SET seconds"); close(fd); return 1; }

    printf("Minutes (0-59)      : "); fflush(stdout); scanf("%hhu", &set_minutes);
    if (ioctl(fd, DS3231_SET_MINUTES, &set_minutes) < 0) { perror("SET minutes"); close(fd); return 1; }

    printf("12h mode (0=24h/1=12h): "); fflush(stdout); scanf("%hhu", &set_12h);
    if (ioctl(fd, DS3231_SET_12H, &set_12h) < 0) { perror("SET 12h"); close(fd); return 1; }

    if (set_12h) {
        printf("Hours (1-12)        : "); fflush(stdout); scanf("%hhu", &set_hours);
        if (ioctl(fd, DS3231_SET_HOURS, &set_hours) < 0) { perror("SET hours"); close(fd); return 1; }

        printf("AM/PM (0=AM/1=PM)   : "); fflush(stdout); scanf("%hhu", &set_pm);
        if (ioctl(fd, DS3231_SET_PM, &set_pm) < 0) { perror("SET PM"); close(fd); return 1; }
    } else {
        printf("Hours (0-23)        : "); fflush(stdout); scanf("%hhu", &set_hours);
        if (ioctl(fd, DS3231_SET_HOURS, &set_hours) < 0) { perror("SET hours"); close(fd); return 1; }
    }

    printf("Day of week (1-7)   : "); fflush(stdout); scanf("%hhu", &set_days);
    if (ioctl(fd, DS3231_SET_DAYS, &set_days) < 0) { perror("SET days"); close(fd); return 1; }

    printf("Date (1-31)         : "); fflush(stdout); scanf("%hhu", &set_date);
    if (ioctl(fd, DS3231_SET_DATE, &set_date) < 0) { perror("SET date"); close(fd); return 1; }

    printf("Month (1-12)        : "); fflush(stdout); scanf("%hhu", &set_month);
    if (ioctl(fd, DS3231_SET_MONTH, &set_month) < 0) { perror("SET month"); close(fd); return 1; }

    printf("Year (1900-2099)    : "); fflush(stdout); scanf("%hu", &set_year);
    if (ioctl(fd, DS3231_SET_YEAR, &set_year) < 0) { perror("SET year"); close(fd); return 1; }

    printf("\n");

    uint8_t  seconds = 0, minutes = 0, hours = 0, pm = 0;
    uint8_t  days = 0, date = 0, month = 0;
    uint16_t year = 0;
    int16_t  temp = 0;

    if (ioctl(fd, DS3231_GET_SECONDS, &seconds) < 0) { perror("GET seconds"); close(fd); return 1; }
    if (ioctl(fd, DS3231_GET_MINUTES, &minutes) < 0) { perror("GET minutes"); close(fd); return 1; }
    if (ioctl(fd, DS3231_GET_HOURS,   &hours)   < 0) { perror("GET hours");   close(fd); return 1; }
    if (ioctl(fd, DS3231_GET_PM,      &pm)      < 0) { perror("GET PM");      close(fd); return 1; }
    if (ioctl(fd, DS3231_GET_DAYS,    &days)    < 0) { perror("GET days");    close(fd); return 1; }
    if (ioctl(fd, DS3231_GET_DATE,    &date)    < 0) { perror("GET date");    close(fd); return 1; }
    if (ioctl(fd, DS3231_GET_MONTH,   &month)   < 0) { perror("GET month");   close(fd); return 1; }
    if (ioctl(fd, DS3231_GET_YEAR,    &year)    < 0) { perror("GET year");    close(fd); return 1; }
    if (ioctl(fd, DS3231_GET_TEMP,    &temp)    < 0) { perror("GET temp");    close(fd); return 1; }
   
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

    if (set_12h)
        printf("%s %02u %s %u  %02u:%02u:%02u %s  %.2f C\n",
               day_str, date, month_str, year,
               hours, minutes, seconds,
               pm ? "PM" : "AM",
               temp / 4.0f);
    else
        printf("%s %02u %s %u  %02u:%02u:%02u  %.2f C\n",
               day_str, date, month_str, year,
               hours, minutes, seconds,
               temp / 4.0f);


    close(fd);
    return 0;
}