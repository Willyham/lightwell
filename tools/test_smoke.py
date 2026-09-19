"""Adversarial evidence tests; no GUI or Pillow installation required for headless checks."""
import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import subprocess
import sys
import dev

class ProcessFailureTests(unittest.TestCase):
    def run_failure(self, effect):
        with tempfile.TemporaryDirectory() as temporary:
            output=Path(temporary)/'evidence ü space'
            with patch('dev.platform.platform',return_value='test'), patch('dev.subprocess.run',side_effect=effect), contextlib.redirect_stdout(io.StringIO()):
                result=dev.smoke(output,'load',Path(__file__))
            self.assertEqual(result,1)
            self.assertEqual(json.loads((output/'result.json').read_text())['status'],'failed')
            self.assertTrue((output/'reproduce.md').is_file())
            self.assertFalse((output/'app/frame-1.png').exists())
            return json.loads((output/'result.json').read_text())
    def test_hung_process_is_failure(self):
        self.run_failure(subprocess.TimeoutExpired('fixture',0.01))
    def test_missing_executable_is_failure(self):
        self.run_failure(FileNotFoundError('missing executable'))
    def test_success_exit_without_evidence_is_failure(self):
        with patch.dict(sys.modules, {'PIL': None}):
            result=self.run_failure(lambda *args,**kwargs: subprocess.CompletedProcess(args,0))
        self.assertIn('result.json',result['error'])

try:
    from PIL import Image
except ImportError:
    Image=None

@unittest.skipIf(Image is None,'Pixel tests use the pinned fixture Python environment')
class PixelFailureTests(unittest.TestCase):
    def test_blank_frame_rejected(self):
        from check_probe_capture import check
        with tempfile.TemporaryDirectory() as temporary:
            path=Path(temporary)/'blank.png'
            Image.new('RGB',(960,640),(20,20,20)).save(path)
            with self.assertRaisesRegex(AssertionError,'blank'):
                check(path)
    def test_stale_frame_and_run_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)
            (root/'events.jsonl').write_text('\n'.join(json.dumps({'event':e,'run_id':'current'}) for e in ['startup','shutdown']))
            result={'status':'captured','run_id':'current','frames':[{'state':{'run_id':'old','requested_generation':1}}]}
            (root/'result.json').write_text(json.dumps(result))
            with self.assertRaisesRegex(AssertionError,'run identity'):
                dev.verify_smoke(root,'load',1)
            result['frames'][0]['state']['run_id']='current'
            result['frames'][0]['state']['requested_generation']=0
            (root/'result.json').write_text(json.dumps(result))
            with self.assertRaisesRegex(AssertionError,'generation'):
                dev.verify_smoke(root,'load',1)

if __name__=='__main__':unittest.main()
