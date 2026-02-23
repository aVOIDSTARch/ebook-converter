/**
 * Ebook Converter C API
 *
 * Link against the static or dynamic library built from the ebook-converter-ffi crate.
 * All path strings are UTF-8. Return values are stable; use these constants instead of raw numbers.
 */

#ifndef EBOOK_CONVERTER_H
#define EBOOK_CONVERTER_H

#ifdef __cplusplus
extern "C" {
#endif

/* Success / validation result */
#define EBOOK_OK                        0
#define EBOOK_VALIDATE_HAS_ERRORS       1   /* ebook_validate: file read ok but validation found errors */

/* Error codes (negative) */
#define EBOOK_ERR_NULL                  -1   /* Null pointer argument */
#define EBOOK_ERR_INVALID_STRING        -2   /* Path string is not valid UTF-8 */
#define EBOOK_ERR_IO                    -3   /* File open/read error (e.g. file not found) */
#define EBOOK_ERR_CONVERT               -3   /* ebook_convert: conversion failed */
#define EBOOK_ERR_DETECT                -4   /* Format detection failed */
#define EBOOK_ERR_READ                  -5   /* Read/parse failed (unsupported format or corrupt) */

/**
 * Convert an ebook file to another format.
 *
 * \param input_path   Path to input file (UTF-8)
 * \param output_path  Path to output file (UTF-8)
 * \param output_format Optional format string ("epub", "txt", etc.); NULL = epub
 * \return EBOOK_OK (0) on success, or a negative EBOOK_ERR_* code
 */
int ebook_convert(const char *input_path, const char *output_path, const char *output_format);

/**
 * Validate an ebook file.
 *
 * \param input_path Path to input file (UTF-8)
 * \return EBOOK_OK (0) if valid, EBOOK_VALIDATE_HAS_ERRORS (1) if validation found errors,
 *         or a negative EBOOK_ERR_* code
 */
int ebook_validate(const char *input_path);

#ifdef __cplusplus
}
#endif

#endif /* EBOOK_CONVERTER_H */
