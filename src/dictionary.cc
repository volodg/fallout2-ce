#include "dictionary.h"

#include <cstdlib>
#include <cstring>

// Migrated
#include "platform_compat.h"

namespace fallout {

// NOTE: I guess this marker is used as a type discriminator for implementing
// nested dictionaries. That's why every dictionary-related function starts
// with a check for this value.
#define DICTIONARY_MARKER 0xFEBAFEBA

static void* dictionaryMallocDefaultImpl(size_t size);
static void* dictionaryReallocDefaultImpl(void* ptr, size_t newSize);
static void dictionaryFreeDefaultImpl(void* ptr);
static int dictionaryFindIndexForKey(Dictionary* dictionary, const char* key, int* index);

// 0x51E408
static MallocProc* gDictionaryMallocProc = dictionaryMallocDefaultImpl;

// 0x51E40C
static ReallocProc* gDictionaryReallocProc = dictionaryReallocDefaultImpl;

// 0x51E410
static FreeProc* gDictionaryFreeProc = dictionaryFreeDefaultImpl;

// 0x4D9B90
static void* dictionaryMallocDefaultImpl(size_t size)
{
    return malloc(size);
}

// 0x4D9B98
static void* dictionaryReallocDefaultImpl(void* ptr, size_t newSize)
{
    return realloc(ptr, newSize);
}

// 0x4D9BA0
static void dictionaryFreeDefaultImpl(void* ptr)
{
    free(ptr);
}

// 0x4D9BA8
int dictionaryInit(Dictionary* dictionary, int initialCapacity, size_t valueSize, DictionaryIO* io)
{
    dictionary->entriesCapacity = initialCapacity;
    dictionary->valueSize = valueSize;
    dictionary->entriesLength = 0;

    if (io != NULL) {
        memcpy(&(dictionary->io), io, sizeof(*io));
    } else {
        dictionary->io.readProc = NULL;
        dictionary->io.writeProc = NULL;
        dictionary->io.field_8 = 0;
        dictionary->io.field_C = 0;
    }

    int rc = 0;

    if (initialCapacity != 0) {
        dictionary->entries = (DictionaryEntry*)gDictionaryMallocProc(sizeof(*dictionary->entries) * initialCapacity);
        if (dictionary->entries == NULL) {
            rc = -1;
        }
    } else {
        dictionary->entries = NULL;
    }

    if (rc != -1) {
        dictionary->marker = DICTIONARY_MARKER;
    }

    return rc;
}

// 0x4D9C0C
int dictionarySetCapacity(Dictionary* dictionary, int newCapacity)
{
    if (dictionary->marker != DICTIONARY_MARKER) {
        return -1;
    }

    if (newCapacity < dictionary->entriesLength) {
        return -1;
    }

    DictionaryEntry* entries = (DictionaryEntry*)gDictionaryReallocProc(dictionary->entries, sizeof(*dictionary->entries) * newCapacity);
    if (entries == NULL) {
        return -1;
    }

    dictionary->entriesCapacity = newCapacity;
    dictionary->entries = entries;

    return 0;
}

// 0x4D9C48
int dictionaryFree(Dictionary* dictionary)
{
    if (dictionary->marker != DICTIONARY_MARKER) {
        return -1;
    }

    for (int index = 0; index < dictionary->entriesLength; index++) {
        DictionaryEntry* entry = &(dictionary->entries[index]);
        if (entry->key != NULL) {
            gDictionaryFreeProc(entry->key);
        }

        if (entry->value != NULL) {
            gDictionaryFreeProc(entry->value);
        }
    }

    if (dictionary->entries != NULL) {
        gDictionaryFreeProc(dictionary->entries);
    }

    memset(dictionary, 0, sizeof(*dictionary));

    return 0;
}

// Finds index for the given key.
//
// Returns 0 if key is found. Otherwise returns -1, in this case [indexPtr]
// specifies an insertion point for given key.
//
// 0x4D9CC4
static int dictionaryFindIndexForKey(Dictionary* dictionary, const char* key, int* indexPtr)
{
    if (dictionary->marker != DICTIONARY_MARKER) {
        return -1;
    }

    if (dictionary->entriesLength == 0) {
        *indexPtr = 0;
        return -1;
    }

    int r = dictionary->entriesLength - 1;
    int l = 0;
    int mid = 0;
    int cmp = 0;
    while (r >= l) {
        mid = (l + r) / 2;

        cmp = compat_stricmp(key, dictionary->entries[mid].key);
        if (cmp == 0) {
            break;
        }

        if (cmp > 0) {
            l = l + 1;
        } else {
            r = r - 1;
        }
    }

    if (cmp == 0) {
        *indexPtr = mid;
        return 0;
    }

    if (cmp < 0) {
        *indexPtr = mid;
    } else {
        *indexPtr = mid + 1;
    }

    return -1;
}

// Returns the index of the entry for the specified key, or -1 if it's not
// present in the dictionary.
//
// 0x4D9D5C
int dictionaryGetIndexByKey(Dictionary* dictionary, const char* key)
{
    if (dictionary->marker != DICTIONARY_MARKER) {
        return -1;
    }

    int index;
    if (dictionaryFindIndexForKey(dictionary, key, &index) != 0) {
        return -1;
    }

    return index;
}

// Adds key-value pair to the dictionary if the specified key is not already
// present.
//
// Returns 0 on success, or -1 on any error (including key already exists
// error).
//
// 0x4D9D88
int dictionaryAddValue(Dictionary* dictionary, const char* key, const void* value)
{
    if (dictionary->marker != DICTIONARY_MARKER) {
        return -1;
    }

    int newElementIndex;
    if (dictionaryFindIndexForKey(dictionary, key, &newElementIndex) == 0) {
        // Element for this key is already exists.
        return -1;
    }

    if (dictionary->entriesLength == dictionary->entriesCapacity) {
        // Dictionary reached it's capacity and needs to be enlarged.
        if (dictionarySetCapacity(dictionary, 2 * (dictionary->entriesCapacity + 1)) == -1) {
            return -1;
        }
    }

    // Make a copy of the key.
    char* keyCopy = (char*)gDictionaryMallocProc(strlen(key) + 1);
    if (keyCopy == NULL) {
        return -1;
    }

    strcpy(keyCopy, key);

    // Make a copy of the value.
    void* valueCopy = NULL;
    if (value != NULL && dictionary->valueSize != 0) {
        valueCopy = gDictionaryMallocProc(dictionary->valueSize);
        if (valueCopy == NULL) {
            gDictionaryFreeProc(keyCopy);
            return -1;
        }
    }

    if (valueCopy != NULL && dictionary->valueSize != 0) {
        memcpy(valueCopy, value, dictionary->valueSize);
    }

    // Starting at the end of entries array loop backwards and move entries down
    // one by one until we reach insertion point.
    for (int index = dictionary->entriesLength; index > newElementIndex; index--) {
        DictionaryEntry* src = &(dictionary->entries[index - 1]);
        DictionaryEntry* dest = &(dictionary->entries[index]);
        memcpy(dest, src, sizeof(*dictionary->entries));
    }

    DictionaryEntry* entry = &(dictionary->entries[newElementIndex]);
    entry->key = keyCopy;
    entry->value = valueCopy;

    dictionary->entriesLength++;

    return 0;
}

// Removes key-value pair from the dictionary if specified key is present in
// the dictionary.
//
// Returns 0 on success, -1 on any error (including key not present error).
//
// 0x4D9EE8
int dictionaryRemoveValue(Dictionary* dictionary, const char* key)
{
    if (dictionary->marker != DICTIONARY_MARKER) {
        return -1;
    }

    int indexToRemove;
    if (dictionaryFindIndexForKey(dictionary, key, &indexToRemove) == -1) {
        return -1;
    }

    DictionaryEntry* entry = &(dictionary->entries[indexToRemove]);

    // Free key and value (which are copies).
    gDictionaryFreeProc(entry->key);
    if (entry->value != NULL) {
        gDictionaryFreeProc(entry->value);
    }

    dictionary->entriesLength--;

    // Starting from the index of the entry we've just removed, loop thru the
    // remaining of the array and move entries up one by one.
    for (int index = indexToRemove; index < dictionary->entriesLength; index++) {
        DictionaryEntry* src = &(dictionary->entries[index + 1]);
        DictionaryEntry* dest = &(dictionary->entries[index]);
        memcpy(dest, src, sizeof(*dictionary->entries));
    }

    return 0;
}

// 0x4DA498
void dictionarySetMemoryProcs(MallocProc* mallocProc, ReallocProc* reallocProc, FreeProc* freeProc)
{
    if (mallocProc != NULL && reallocProc != NULL && freeProc != NULL) {
        gDictionaryMallocProc = mallocProc;
        gDictionaryReallocProc = reallocProc;
        gDictionaryFreeProc = freeProc;
    } else {
        gDictionaryMallocProc = dictionaryMallocDefaultImpl;
        gDictionaryReallocProc = dictionaryReallocDefaultImpl;
        gDictionaryFreeProc = dictionaryFreeDefaultImpl;
    }
}

} // namespace fallout
