# -*- coding: utf-8 -*-
import gzip
import sys

import requests


def coveralls(data):
    r = requests.post('https://coveralls.io/api/v1/jobs', files={
        'json_file': ('json_file', gzip.compress(data), 'gzip/json')
    })

    try:
        result = r.json()
    except ValueError:
        raise Exception('Failure to submit data. Response [%s]: %s' % (r.status_code, r.text))

    return result['url'] + '.json'

def main():
    res=coveralls(open(sys.argv[1], "rb").read())
    print(res)

if __name__=="__main__":
    main()
