# importer.py
# SPDX-FileCopyrightText: 2023 nate-xyz
# SPDX-License-Identifier: GPL-3.0-or-later

import os
import pprint

from extracting import translate, get_mutagen, register_formats, cover_art, translate_and_cover_art

pp = pprint.PrettyPrinter(indent=4)

from tqdm import tqdm

import traceback
import sys 

from loguru import logger


#importer will scan a music folder for all valid tracks and load them into the internal model
class Importer():
    def __init__(self) -> None:
        super().__init__()
        logger.debug('Importer init')
        
    def load_folder(self, directory) -> dict:
        logger.debug('\tload_folder')
        song_uris = set()
        for root, _, files in os.walk(directory): 
            for filename in files:
                if self.is_song(filename) and not filename.startswith('.'):
                    f = os.path.join(root, filename) 
                    song_uris.add(f)
        return self.get_metadata(song_uris)

    def load_folder_coverart(self, directory) -> dict:
        logger.debug('\tload_folder')
        song_uris = set()
        for root, _, files in os.walk(directory): 
            for filename in files:
                if self.is_song(filename) and not filename.startswith('.'):
                    f = os.path.join(root, filename) 
                    song_uris.add(f)
        return self.get_coverart(song_uris)

    def load_folder_both(self, directory) -> dict:
        logger.debug('\tload_folder')
        song_uris = set()
        for root, _, files in os.walk(directory): 
            for filename in files:
                if self.is_song(filename) and not filename.startswith('.'):
                    f = os.path.join(root, filename) 
                    song_uris.add(f)
        return self.get_both(song_uris)

    def get_both(self, uris) -> dict:
        logger.debug('\tget_metadata')
        TAGS = dict()
        COVER_ART = dict()
        register_formats()
        
        results = []
        logger.debug('\tget_metadata -> retrieving mutagen files')
        for song_uri in tqdm(uris):
            results.append(get_mutagen_file(song_uri))
        
        logger.debug('get_metadata -> getting coverart from mutagen files')
        for song_uri, mutagen_file in tqdm(results):
            if mutagen_file != None:    
                try:
                    tags, cover_art = translate_and_cover_art(song_uri, mutagen_file)  
                    if tags:   
                        TAGS[song_uri] = tags

                    if cover_art:
                        if type(cover_art) != bytes:
                            COVER_ART[song_uri] = bytes(cover_art)
                        else:
                            COVER_ART[song_uri] = cover_art
                except:
                    logger.error(f'exception on cover art extract: {song_uri}')
                    exc_type, exc_value, exc_tb = sys.exc_info()
                    logger.error(traceback.format_exception(exc_type, exc_value, exc_tb))

        return (TAGS, COVER_ART)

    def get_coverart(self, uris) -> dict:
        logger.debug('\tget_metadata')
        COVER_ART = dict()
        register_formats()
        
        results = []
        logger.debug('\tget_metadata -> retrieving mutagen files')
        prog = tqdm(uris, ascii=False)
        prog.set_description("Loading files (1/2)")
        for song_uri in prog:
            results.append(get_mutagen_file(song_uri))
        
        logger.debug('get_metadata -> getting coverart from mutagen files')
        
        prog = tqdm(results, ascii=False)
        prog.set_description("Extracting tags (1/2)")
        for song_uri, mutagen_file in prog:
            if mutagen_file != None:    
                try:
                    data = cover_art(song_uri, mutagen_file)  
                    if data:
                        if type(data) != bytes:
                            COVER_ART[song_uri] = bytes(data)
                        else:
                            COVER_ART[song_uri] = data
                except:
                    logger.error(f'exception on cover art extract: {song_uri}')
                    exc_type, exc_value, exc_tb = sys.exc_info()
                    logger.error(traceback.format_exception(exc_type, exc_value, exc_tb))

        return COVER_ART

    def get_metadata(self, uris) -> dict:
        logger.debug('\tget_metadata')
        ALL_TAGS = dict()
        register_formats()
        
        results = []
        logger.debug('\tget_metadata -> retrieving mutagen files')
        for song_uri in tqdm(uris):
            results.append(get_mutagen_file(song_uri))
        
        logger.debug('get_metadata -> getting tags from mutagen files')
        for song_uri, mutagen_file in tqdm(results):
            if mutagen_file != None:
                try:
                    tags = translate(song_uri, mutagen_file)  
                    if tags:   
                        ALL_TAGS[song_uri] = tags
                    else:
                        logger.error(f"Unable to extract {song_uri}, no tags.")
                except:
                    logger.error(f'exception on translate: {song_uri}')
                    exc_type, exc_value, exc_tb = sys.exc_info()
                    logger.error(traceback.format_exception(exc_type, exc_value, exc_tb))

        return ALL_TAGS

    def get_metadata(self, uris) -> dict:
        logger.debug('\tget_metadata')
        ALL_TAGS = dict()
        register_formats()
        
        results = []
        logger.debug('\tget_metadata -> retrieving mutagen files')
        for song_uri in tqdm(uris):
            results.append(get_mutagen_file(song_uri))
        
        logger.debug('get_metadata -> getting tags from mutagen files')
        for song_uri, mutagen_file in tqdm(results):
            if mutagen_file != None:
                try:
                    tags = translate(song_uri, mutagen_file)  
                    if tags:   
                        ALL_TAGS[song_uri] = tags
                    else:
                        logger.error(f"Unable to extract {song_uri}, no tags.")
                except:
                    logger.error(f'exception on translate: {song_uri}')
                    exc_type, exc_value, exc_tb = sys.exc_info()
                    logger.error(traceback.format_exception(exc_type, exc_value, exc_tb))

        return ALL_TAGS
    def is_song(self, filepath):
        filetype = os.path.splitext(filepath)[1]
        return filetype in {".mp3", ".aif", ".ogg", ".opus", ".flac", ".flac", ".mp4", ".m4a", ".asf", ".wma", ".wmv"}


def get_mutagen_file(song_uri):
    return song_uri, get_mutagen(song_uri)