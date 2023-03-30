# util.py
# SPDX-FileCopyrightText: 2023 nate-xyz
# SPDX-License-Identifier: GPL-3.0-or-later

import re

def seconds_to_string(duration):
    """Convert a time in seconds to a mm:ss string

    :param int duration: Time in seconds
    :return: Time in mm:ss format
    :rtype: str
    """
    seconds = duration
    minutes = seconds // 60
    seconds %= 60

    return '{:d}:{:02d}'.format(int(minutes), int(seconds))

def seconds_to_string_longform(duration):
    minutes, seconds = divmod(duration, 60)
    hours, minutes = divmod(minutes, 60)
    
    if hours > 0:
        if hours < 2:
            return '{:d} hour and {:02d} minutes'.format(int(hours), int(minutes))
        else:
            return '{:d} hours and {:02d} minutes'.format(int(hours), int(minutes))
    else:
        return '{:d} minutes'.format(int(minutes))

def clamp_float(value):
    if value < 0: value=0 
    if value > 1.0: value = 1.0
    return value

def get_tag(tags: dict, tag_name: str):
    tag = tags[tag_name]
    if isinstance(tag, list):
        if len(tag) == 0:
            return ''
        else:
            return tag[0]
    else:
        return tag

def get_number_tag(tags: dict, tag_name: str):
    tag = tags[tag_name]
    if isinstance(tag, list):
        tag = tag[0]

    if "/" in tag:
        tag = tag.split("/")[0]
    tag = remove_non_digits(tag)
    return int(tag)   

def remove_non_digits(s):
    return re.sub(r'\D', '', s)