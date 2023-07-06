#ifndef DATAFILE_H
#define DATAFILE_H

namespace fallout {

typedef unsigned char*(DatafileLoader)(char* path, unsigned char* palette, int* widthPtr, int* heightPtr);
typedef char*(DatafileNameMangler)(char* path);

extern DatafileLoader* gDatafileLoader;
extern DatafileNameMangler* gDatafileNameMangler;

extern unsigned char gDatafilePalette[768];

char* datafileDefaultNameManglerImpl(char* path);
void sub_42EE84(unsigned char* data, unsigned char* palette, int width, int height);
unsigned char* datafileReadRaw(char* path, int* widthPtr, int* heightPtr);
unsigned char* datafileRead(char* path, int* widthPtr, int* heightPtr);
unsigned char* datafileGetPalette();

} // namespace fallout

#endif /* DATAFILE_H */
