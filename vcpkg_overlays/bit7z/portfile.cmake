vcpkg_minimum_required(VERSION 2024-01-01)

vcpkg_from_github(
    OUT_SOURCE_PATH SOURCE_PATH
    REPO rikyoz/bit7z
    REF v4.1.0
    SHA512 d240997e3b1f6eb8d0b19c89bf3b12044cbb10ba495b4ba535efc1cd04390157031a303025819b6fd9a6a51bdca7b59ad50df45055cbde9130ffd4c8279a0863
    HEAD_REF develop
)

# Create DOC/readme.txt for 7-zip version detection
set(_7ZIP_DOC_DIR "${CURRENT_INSTALLED_DIR}/include/7zip/DOC")
file(MAKE_DIRECTORY "${_7ZIP_DOC_DIR}")
file(WRITE "${_7ZIP_DOC_DIR}/readme.txt" "7-Zip 26.01\n")

vcpkg_cmake_configure(
    SOURCE_PATH "${SOURCE_PATH}"
    OPTIONS
        -DBIT7Z_AUTO_FORMAT:BOOL=OFF
        -DBIT7Z_REGEX_MATCHING:BOOL=OFF
        -DBIT7Z_CUSTOM_7ZIP_PATH:String=${CURRENT_INSTALLED_DIR}/include/7zip
)

vcpkg_cmake_install()

file(INSTALL "${SOURCE_PATH}/LICENSE" DESTINATION "${CURRENT_PACKAGES_DIR}/share/${PORT}" RENAME copyright)
