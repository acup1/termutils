from setuptools import setup
from pybind11.setup_helpers import Pybind11Extension, build_ext
from pybind11 import get_include

ext_modules = [
    Pybind11Extension(
        "termutils",  # имя модуля: Python будет импортировать как import _test
        ["termutils.c", "bindings.cpp"],
        include_dirs=[get_include()],
        language="c++",
        cxx_std=11,
    ),
]

setup(
    name="termutils",
    version="1.0.0",
    author="acup",
    author_email="georgiyegoriy@gmail.com",
    description="lib for working with raw terminal and windows",
    ext_modules=ext_modules,
    cmdclass={"build_ext": build_ext},
    zip_safe=False,
    python_requires=">=3.6",
)
