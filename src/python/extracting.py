# extracting.py
# SPDX-FileCopyrightText: 2023 nate-xyz
# SPDX-License-Identifier: GPL-3.0-or-later

import os

from pathlib import Path

import mutagen.flac
import base64
#from mutagen import mp3, oggvorbis, oggopus, flac, mp4, asf
from mutagen.mp3 import MP3
from mutagen.flac import FLAC
from mutagen.oggvorbis import OggVorbis
from mutagen.oggopus import OggOpus
from mutagen.mp4 import MP4
from mutagen.asf import ASF

from typing import Optional

import imghdr
from translate_dicts import *

from loguru import logger

from collections import defaultdict

import traceback

_extensions = {}

def register_formats():
    for filetype in {".mp3", ".aif", ".ogg", ".opus", ".flac", ".flac", ".mp4", ".m4a", ".asf", ".wma", ".wmv"}:
        if filetype in {'.mp3', '.aif'}:
            Kind = MP3
        elif filetype == '.flac':
            Kind = FLAC
        elif filetype in {'.mp4', '.m4a'}:
            Kind = MP4
        elif filetype == '.ogg':
            Kind = OggVorbis
        elif filetype == '.opus':
            Kind = OggOpus
        elif filetype in {'.asf', '.wma', '.wmv'}:
            Kind = ASF
        else:
            continue
        _extensions[filetype] = Kind

def get_mutagen(uri):
    try:
        mutagen_file = None
        with open(uri, "rb", buffering=0) as file: 
            filetype = os.path.splitext(uri)[1]

            if filetype in {'.mp3', '.aif'}:
                mutagen_file = MP3(fileobj=file)
            elif filetype == '.flac':
                mutagen_file = FLAC(fileobj=file)
            elif filetype in {'.mp4', '.m4a'}:
                mutagen_file = MP4(fileobj=file)
            elif filetype == '.ogg':
                mutagen_file = OggVorbis(fileobj=file)
            elif filetype == '.opus':
                mutagen_file = OggOpus(fileobj=file)
            elif filetype in {'.asf', '.wma', '.wmv'}:
                mutagen_file = ASF(fileobj=file)

        if mutagen_file == None:
            logger.error("(NONE) Could not parse: {}".format( uri))
            return None
        else:
            return mutagen_file
    
    except Exception:
        logger.error("(EXCEPTION) Could not parse: {}".format( uri))
        return None

def translate_and_cover_art(filepath, mutagen_file):
    return (translate(filepath, mutagen_file), cover_art(filepath, mutagen_file))

def translate(filepath, mutagen_file) -> Optional[dict]:
    if mutagen_file.tags == None:
        return None

    tag_mappings = {
        '.mp4':     mp4_translate,
        '.mp3':     id3_translate,
        '.wma':     asf_translate,
    }
    tag_mappings['.m4a'] = tag_mappings['.mp4']
    tag_mappings['.aif'] = tag_mappings['.mp3']
    tag_mappings['.asf'] = tag_mappings['.wma']
    tag_mappings['.wmv'] = tag_mappings['.wma']

    filetype = os.path.splitext(filepath)[1]
   
    RETURN_MAP = dict()

    if filetype in {".ogg", ".flac", ".opus"}:
        for _tup in mutagen_file.tags:
          # logger.debug(f"{_tup[0].lower()} : {_tup[1]} {type(_tup[1])}")
            #RETURN_MAP[_tup[0].lower()] = _tup[1]
            logger.debug(f"FLAC tag: {_tup[0].lower()} -> {_tup[1]}")

            add_to_map(RETURN_MAP, _tup[0].lower(), [_tup[1]])
    else:
        for tag in tag_mappings[filetype].keys():
            value = None

            for _key in mutagen_file.tags.keys():
                _tag = _key
                if ":" in _key:
                    _tag = _key.split(":")[0]

                if tag == _tag:
                    if filetype in ['.mp3', '.aif']:
                        try:
                            value = mutagen_file.tags[_key].text
                        except:
                            value = mutagen_file.tags[_key]
                    else:
                        value = mutagen_file.tags[_key]

                    
                    value = [str(i) for i in value if isinstance(value, list)]


            if value != None:
              # logger.debug(f"{tag_mappings[filetype][tag]} : {value} {type(value)}")
                #RETURN_MAP[tag_mappings[filetype][tag]] = value
                add_to_map(RETURN_MAP, tag_mappings[filetype][tag], value)

        if filetype in ['.mp4', '.m4a']:
            for _key in mutagen_file.tags.keys():
                value = None
                _tag = _key
                if ":" in _key:
                    _tag = _key.split(":")[0]

                if _tag == 'disk':
                    _tup = mutagen_file.tags[_key][0]
                    if len(_tup) < 2:
                        continue
                    discnumber = "{}/{}".format(_tup[0], _tup[1]) 
                    #RETURN_MAP['discnumber'] = discnumber 
                    add_to_map(RETURN_MAP, 'discnumber', discnumber)
                if _tag == 'trkn':
                    _tup = mutagen_file.tags[_key][0]
                    if len(_tup) < 2:
                        continue
                    tracknumber = "{}/{}".format(_tup[0], _tup[1])
                    #RETURN_MAP['tracknumber'] = tracknumber
                    add_to_map(RETURN_MAP, 'tracknumber', tracknumber)

                if value != None:
                    #RETURN_MAP[_tag] = value
                    add_to_map(RETURN_MAP, _tag, value)

    #RETURN_MAP['duration'] = mutagen_file.info.length
    #RETURN_MAP['filetype_'] = filetype
    #RETURN_MAP['coverart'] = extract_coverart(filetype, mutagen_file)

    add_to_map(RETURN_MAP, 'duration', mutagen_file.info.length)
    add_to_map(RETURN_MAP, 'filetype_', filetype)
    # coverart = extract_coverart(filetype, mutagen_file)
    # if coverart:
    #     add_to_map(RETURN_MAP, 'coverart', coverart)
    # add_to_map(RETURN_MAP, 'coverart', extract_coverart(filetype, mutagen_file))
    
    # else:
    #     ret, data = get_coverart_from_file_in_folder(filepath, filetype)
    #     if ret:
    #         RETURN_MAP['coverart'] = data

    #ensure tag mapping has essential tags:
    # - title
    # - albumartist / artist 
    # - album

    if 'str_list' not in RETURN_MAP:
        RETURN_MAP['str_list'] = dict()

    if 'title' not in RETURN_MAP['str_list']:
        #RETURN_MAP['title'] = os.path.basename(filepath)
        add_to_map(RETURN_MAP, 'title', [os.path.basename(filepath)])
    
    if 'albumartist' not in RETURN_MAP['str_list']:
        if 'artist' not in RETURN_MAP['str_list']:
            #RETURN_MAP['albumartist'] = "Unknown Artist"
            add_to_map(RETURN_MAP, 'albumartist', ["Unknown Artist"])
        else:
            add_to_map(RETURN_MAP, 'albumartist', RETURN_MAP['str_list']['artist'])

    if 'album' not in RETURN_MAP['str_list']:
        #RETURN_MAP['album'] = "Unknown Album"
        add_to_map(RETURN_MAP, 'album', ["Unknown Album"])

    return RETURN_MAP

def cover_art(filepath, mutagen_file) -> Optional[bytes]:
    filetype = os.path.splitext(filepath)[1]
    cover_art_data = extract_coverart(filetype, mutagen_file)
    if cover_art_data:
        return cover_art_data

def add_to_map(return_map, key, value):
    # logger.debug(f"Adding to map: {key} {value}")
    try:
        if type(value) == list:
            _dict = defaultdict(list)
            for subvalue in value:
                # logger.debug(f"adding to subdict {str(type(subvalue).__name__)} {subvalue}")
                _dict[str(type(subvalue).__name__)].append(subvalue)
            # logger.debug("subdict done")
            for type_key, value_array in _dict.items():
                type_list_key = f"{type_key}_list"
              # logger.debug(f"subdict {type_list_key} {key} {value_array}")
                if type_list_key not in return_map:
                    return_map[type_list_key] = dict()

                return_map[type_list_key][key] = value_array
        else:
            type_key = str(type(value).__name__)
            # logger.debug(f"Adding to map: {type_key} {key} {value}")
            if type_key not in return_map:
                return_map[type_key] = dict()
            return_map[type_key][key] = value
    
    except Exception as e:
        logger.error(f"Add to map error: {e}")
        logger.error(traceback.format_exc())
        os._exit(1)

def extract_coverart(filetype, metadata) -> Optional[bytes] :
    tag_mappings = {
    '.mp4': { 'covr':    'coverart' },
    '.mp3': { 'APIC': 'coverart' },
    '.ogg': { 'metadata_block_picture': 'coverart' },
    '.wma' : {'WM/Picture': 'coverart'},
    }
    tag_mappings['.m4a']  = tag_mappings['.mp4']
    tag_mappings['.opus'] = tag_mappings['.ogg']
    tag_mappings['.flac'] = tag_mappings['.ogg']
    tag_mappings['.aif']  = tag_mappings['.mp3']
    tag_mappings['.asf'] = tag_mappings['.wma']
    tag_mappings['.wmv'] = tag_mappings['.wma']
    
    if filetype == '.flac' and metadata.pictures:
        return metadata.pictures[0].data
  
    for tag in tag_mappings[filetype].keys():
        value = None
        
        #get tag 
        if filetype in ["ogg", "flac", "opus"]:
            for _tup in metadata.tags:
                if tag == _tup[0].lower():
                    value = _tup[1]
        else:
            for _key in metadata.tags.keys():
                _tag = _key
                if ":" in _key:
                    _tag = _key.split(":")[0]
                if tag == _tag:
                    value = metadata.tags[_key]
                    if isinstance(value, list):
                        value = value[0]
        
        if value and tag_mappings[filetype][tag] == 'coverart':
            if filetype in [".ogg", ".opus"]:
                pic = mutagen.flac.Picture(base64.b64decode(value))
                return pic.data

            if filetype in [".mp4", ".m4a"]:
                if imghdr.what(None, h=value) == None:
                  # logger.debug('error on mp4 image')
                    return None
                return value

            if filetype in ['.asf', '.wma', '.wmv']:
                #return value.value
                return None

            # some coverart classes store the image in the data attribute whereas others do not
            if hasattr( value, 'data' ):
                if imghdr.what(None, h=value.data) == None:
                    return None
                return value.data

    return None

def get_coverart_from_file_in_folder(filepath, filetype):
    parent = Path(filepath).parent
    art_path = parent / 'cover.jpg'
    if art_path.is_file():
        return art_path.read_bytes()
    return None