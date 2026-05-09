import runpy, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
runpy.run_path('C:\\Proyectos\\Pelendur\\scripts\\testing\\comprehensive_test.py')
